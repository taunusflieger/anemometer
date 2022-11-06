#![cfg_attr(debug_assertions, allow(dead_code))]

use core::str;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use log::info;
use std::{thread::sleep, time::Duration};

use embedded_svc::{
    io::Write,
    utils::http::Headers,
    wifi::{self, AuthMethod, ClientConfiguration},
};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    netif::IpEvent,
    nvs::EspDefaultNvsPartition,
    wifi::{EspWifi, WifiEvent, WifiWait},
};
use esp_idf_sys as _;
use esp_idf_sys::*;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::sync::mpsc;
const OTA_PAGE: &str = include_str!("../html/ota-update.html");

const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

enum SysLoopMsg {
    WifiDisconnect,
    IpAddressAsquired,
}

struct LazyInitHttpServer {
    data: Rc<RefCell<Option<EspHttpServer>>>,
}

impl LazyInitHttpServer {
    fn new() -> Self {
        Self {
            data: Rc::new(RefCell::new(None)),
        }
    }
    fn create(&self, conf: &Configuration) -> RefMut<'_, EspHttpServer> {
        if self.data.borrow().is_none() {
            *self.data.borrow_mut() = Some(EspHttpServer::new(conf).unwrap());
        }
        let m = self.data.borrow_mut();
        RefMut::map(m, |m| m.as_mut().unwrap())
    }
    fn get(&self) -> Option<RefMut<'_, EspHttpServer>> {
        let m = self.data.borrow_mut();
        if m.is_some() {
            Some(RefMut::map(m, |m| m.as_mut().unwrap()))
        } else {
            None
        }
    }
    fn clear(&self) {
        *self.data.borrow_mut() = None;
    }
    /*
    fn ref_count(&self) -> usize {
        Rc::strong_count(&self.data)
    }
    */
}

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let httpd = LazyInitHttpServer::new();
    let (tx, rx) = mpsc::channel::<SysLoopMsg>();

    println!("Wifi name {}", CONFIG.wifi_ssid);
    let mut auth_method = AuthMethod::WPA2WPA3Personal; // Todo: add this setting - router dependent
    if CONFIG.wifi_ssid.is_empty() {
        anyhow::bail!("missing WiFi name")
    }
    if CONFIG.wifi_psk.is_empty() {
        auth_method = AuthMethod::None;
        info!("Wifi password is empty");
    }

    let peripherals = Peripherals::take().unwrap();
    let nvs_default_partition = EspDefaultNvsPartition::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    let mut wifi = EspWifi::new(
        peripherals.modem,
        sysloop.clone(),
        Some(nvs_default_partition),
    )?;

    wifi.set_configuration(&wifi::Configuration::Client(ClientConfiguration {
        ssid: CONFIG.wifi_ssid.into(),
        password: CONFIG.wifi_psk.into(),
        auth_method,
        ..Default::default()
    }))?;

    let wait = WifiWait::new(&sysloop)?;

    wifi.start()?;

    // disable power save
    esp!(unsafe { esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE) })?;

    wait.wait(|| wifi.is_started().unwrap());

    sleep(Duration::from_millis(2000));

    info!("Wifi started");
    wifi.connect()?;

    let tx1 = tx.clone();
    let _wifi_event_sub = sysloop.subscribe(move |event: &WifiEvent| match event {
        WifiEvent::StaConnected => {
            info!("******* Received STA Connected Event");
        }
        WifiEvent::StaDisconnected => {
            info!("******* Received STA Disconnected event");
            tx.send(SysLoopMsg::WifiDisconnect)
                .expect("wifi event channel closed");
            //    sleep(Duration::from_millis(10));
            if let Err(err) = wifi.connect() {
                info!("Error calling wifi.connect in wifi reconnect {:?}", err);
            }
        }
        _ => info!("Received other Wifi event"),
    })?;

    let _ip_event_sub = sysloop.subscribe(move |event: &IpEvent| match event {
        IpEvent::DhcpIpAssigned(_assignment) => {
            info!("************ Received IPEvent address assigned");
            tx1.send(SysLoopMsg::IpAddressAsquired)
                .expect("IP event channel closed");
        }
        _ => info!("Received other IPEvent"),
    })?;

    loop {
        match rx.try_recv() {
            Ok(SysLoopMsg::WifiDisconnect) => {
                info!("mpsc loop: WifiDisconnect received");
                httpd.clear();
            }
            Ok(SysLoopMsg::IpAddressAsquired) => {
                info!("mpsc loop: IpAddressAsquired received");

                // test remove
                sleep(Duration::from_millis(1000));

                let server_config = Configuration::default();
                let mut s = httpd.create(&server_config);

                if let Err(err) = s.fn_handler("/", embedded_svc::http::Method::Get, move |req| {
                    let mut headers = Headers::<1>::new();
                    headers.set_cache_control("no-store");

                    let mut response = req.into_response(200, None, headers.as_slice())?;
                    response.write_all(OTA_PAGE.as_bytes())?;
                    info!("Processing '/' request");
                    Ok(())
                }) {
                    info!(
                        "mpsc loop: failed to register http handler /temperature: {:?} - restarting device",
                        err
                    );
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }

                if let Err(err) = s.fn_handler(
                    "/api/version",
                    embedded_svc::http::Method::Get,
                    move |req| {
                        let mut headers = Headers::<1>::new();
                        headers.set_cache_control("no-store");

                        let mut resp = req.into_response(200, None, headers.as_slice())?;
                        resp.write_all(FIRMWARE_VERSION.as_bytes())?;
                        info!("Processing '/api/version' request");
                        Ok(())
                    },
                ) {
                    info!(
                        "mpsc loop: failed to register http handler /api/version: {:?} - restarting device",
                        err
                    );
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }

                if let Err(err) = s.fn_handler(
                    "/api/ota",
                    embedded_svc::http::Method::Post,
                    move |mut req| {
                        use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
                        use esp_idf_svc::ota::EspOta;

                        const BUF_MAX: usize = 2 * 1024;
                        const MAX_RETRY: u8 = 3;
                        let mut firmware_update_ok = false;

                        info!("Start processing /api/ota");

                        let mut content_length: usize = 0;
                        let mut body: [u8; BUF_MAX] = [0; BUF_MAX];
                        let mut headers = Headers::<1>::new();
                        headers.set_cache_control("no-store");

                        let res = req.connection().read(&mut body);
                        info!("POST body size: {}", res.unwrap());

                        // TODO: check error handling!
                        let firmware_url = url::form_urlencoded::parse(&body)
                            .filter(|p| p.0 == "firmware")
                            .map(|p| p.1)
                            .next()
                            .ok_or_else(|| anyhow::anyhow!("No parameter firmware"));

                        let firmware_url = firmware_url.unwrap().to_string();
                        let firmware_url = firmware_url.trim_matches(char::from(0));
                        info!("Will use firmware from: {}", firmware_url);

                        let mut ota = EspOta::new().unwrap();

                        let ota_update = ota.initiate_update().unwrap();
                        info!("EspOta created");

                        let mut client = EspHttpConnection::new(&Configuration {
                            buffer_size: Some(BUF_MAX),
                            ..Default::default()
                        })
                        .expect("creation of EspHttpConnection should have worked");

                        info!("EspHttpConnection created");
                        let _resp = client.initiate_request(
                            embedded_svc::http::Method::Get,
                            firmware_url,
                            &[],
                        );

                        info!("after client.initiate_request()");

                        client.initiate_response()?;

                        if let Some(len) = client.header("Content-Length") {
                            content_length = len.parse().unwrap();
                        } else {
                            info!("reading content length for firmware update http request failed");
                        }

                        info!("Content-length: {:?}", content_length);

                        info!(">>>>>>>>>>>>>>>> initiating OTA update");

                        let mut bytes_read_total = 0;

                        loop {
                            esp_idf_hal::delay::FreeRtos::delay_ms(10);
                            let n_bytes_read = match client.read(&mut body) {
                                Ok(n) => n,
                                Err(err) => {
                                    info!("ERROR reading firmware batch {:?}", err);
                                    break;
                                }
                            };
                            bytes_read_total += n_bytes_read;

                            if !body.is_empty() {
                                match ota_update.write(&body) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        info!("ERROR failed to write update with: {:?}", err);
                                        break;
                                    }
                                }
                            } else {
                                info!("ERROR firmware image with zero length");
                                break;
                            }

                            if body.len() > n_bytes_read {
                                break;
                            }
                        }

                        if bytes_read_total == content_length {
                            firmware_update_ok = true;
                        }

                        let confirmation_msg = if firmware_update_ok {
                            ota_update.complete().unwrap();
                            info!("completed firmware update");

                            templated("Successfully completed firmware update")
                        } else {
                            ota_update.abort().unwrap();
                            info!("ERROR firmware update failed");
                            templated("Firmare update failed")
                        };

                        let mut response = req.into_response(200, None, headers.as_slice())?;
                        response.write_all(confirmation_msg.as_bytes())?;

                        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
                        info!("restarting device after firmware update");
                        unsafe {
                            esp_idf_sys::esp_restart();
                        }
                        info!("failed to restart device");
                        Ok(())
                    },
                ) {
                    info!(
                        "mpsc loop: failed to register http handler /api/ota: {:?} - restarting device",
                        err
                    );
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }

                if let Err(err) = s.fn_handler(
                    "/temperature",
                    embedded_svc::http::Method::Get,
                    move |req| {
                        let temp_val = 42.0;
                        let html = temperature(temp_val);
                        let mut headers = Headers::<1>::new();
                        headers.set_cache_control("no-store");

                        let mut resp = req.into_response(200, None, headers.as_slice())?;
                        resp.write_all(html.as_bytes())?;
                        info!("Processing '/temperature' request");
                        Ok(())
                    },
                ) {
                    info!(
                        "mpsc loop: failed to register http handler /temperature: {:?} - restarting device",
                        err
                    );
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }
            }
            Err(err) => {
                if err == mpsc::TryRecvError::Disconnected {
                    info!("mpsc loop: error recv {:?} - restarting device", err);
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                } // the other error value is Empty which is okay and we ignore
            }
        }

        esp_idf_hal::delay::FreeRtos::delay_ms(100);
    }
}

fn templated(content: impl AsRef<str>) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>esp-rs web server</title>
    </head>
    <body>
        {}
    </body>
</html>
"#,
        content.as_ref()
    )
}

fn temperature(val: f32) -> String {
    templated(format!("chip temperature: {:.2}°C", val))
}

fn wind_html(speed: u8, direction: u16) -> String {
    templated(format!(
        "wind speed: {} m/s<p>wind direction: {}",
        speed, direction
    ))
}
