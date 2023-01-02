use channel_bridge::{asynch::pubsub, asynch::*};
use embedded_svc::mqtt::client::asynch::{Client, Connection, Publish};
use embedded_svc::utils::asyncify::Asyncify;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, Wifi as WifiTrait};
use esp_idf_hal::modem::WifiModemPeripheral;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{EspWifi, WifiEvent};
use esp_idf_sys::EspError;
use log::*;

use crate::errors::*;
use crate::mqtt_msg::*;

const SSID: &str = env!("RUST_ESP32_ANEMOMETER_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_ANEMOMETER_WIFI_PASS");

pub fn wifi<'d>(
    modem: impl Peripheral<P = impl WifiModemPeripheral + 'd> + 'd,
    mut sysloop: EspSystemEventLoop,
    partition: Option<EspDefaultNvsPartition>,
) -> Result<(impl WifiTrait + 'd, impl Receiver<Data = WifiEvent>), EspError> {
    let mut wifi = EspWifi::new(modem, sysloop.clone(), partition)?;

    info!("Wifi name {}", SSID);

    if PASS.is_empty() {
        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: SSID.into(),
            auth_method: AuthMethod::None,
            ..Default::default()
        }))?;
    } else {
        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: SSID.into(),
            password: PASS.into(),
            ..Default::default()
        }))?;
    }

    wifi.start()?;

    wifi.connect()?;

    Ok((
        wifi,
        pubsub::SvcReceiver::new(sysloop.as_async().subscribe()?),
    ))
}

pub fn mqtt() -> Result<
    (
        &'static str,
        impl Client + Publish,
        impl Connection<Message = Option<MqttCommand>>,
    ),
    InitError,
> {
    let client_id = "anemometer";
    let mut mqtt_parser = MessageParser::new();

    let (mqtt_client, mqtt_conn) = EspMqttClient::new_with_converting_async_conn(
        "mqtt://192.168.100.86:1883",
        &MqttClientConfiguration {
            client_id: Some(client_id),
            ..Default::default()
        },
        move |event| mqtt_parser.convert(event),
    )?;

    let mqtt_client = mqtt_client.into_async();

    Ok((client_id, mqtt_client, mqtt_conn))
}

pub mod http {
    use std::cell::{RefCell, RefMut};
    use std::rc::Rc;

    use esp_idf_svc::http::server::{Configuration, EspHttpServer};

    pub struct LazyInitHttpServer {
        data: Rc<RefCell<Option<EspHttpServer>>>,
    }

    impl LazyInitHttpServer {
        pub fn new() -> Self {
            Self {
                data: Rc::new(RefCell::new(None)),
            }
        }
        pub fn create(&self, conf: &Configuration) -> RefMut<'_, EspHttpServer> {
            if self.data.borrow().is_none() {
                *self.data.borrow_mut() = Some(EspHttpServer::new(conf).unwrap());
            }
            let m = self.data.borrow_mut();
            RefMut::map(m, |m| m.as_mut().unwrap())
        }

        #[allow(dead_code)]
        pub fn get(&self) -> Option<RefMut<'_, EspHttpServer>> {
            let m = self.data.borrow_mut();
            if m.is_some() {
                Some(RefMut::map(m, |m| m.as_mut().unwrap()))
            } else {
                None
            }
        }

        pub fn clear(&self) {
            *self.data.borrow_mut() = None;
        }

        #[allow(dead_code)]
        fn ref_count(&self) -> usize {
            Rc::strong_count(&self.data)
        }
    }
}
