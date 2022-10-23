#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use core::str;
use embedded_svc::io::adapters::ToStd;
use embedded_svc::timer::*;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::timer::*;
use log::info;
use std::io::Read;
use std::{
    sync::atomic::{AtomicU16, AtomicU8, Ordering},
    sync::{Arc, Condvar, Mutex},
    thread::sleep,
    time::Duration,
};

use embedded_svc::http::server::{Connection, HandlerResult, Request};
use embedded_svc::utils::http::Headers;
use embedded_svc::wifi::{self, AuthMethod, ClientConfiguration};

use embedded_svc::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::{
    netif::IpEvent, nvs::EspDefaultNvs, wifi::EspWifi, wifi::WifiEvent, wifi::WifiWait,
};
use esp_idf_sys as _;
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
        auth_method: auth_method,
        ..Default::default()
    }))?;

    let wait = WifiWait::new(&sysloop)?;

    wifi.start()?;

    wait.wait(|| wifi.is_started().unwrap());

    info!("Wifi started");
    wifi.connect()?;

    let tx = tx.clone();
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
                        "mpsc loop: failed to register http handler /temperature: {:?}",
                        err
                    );
                    info!("mpsc loop: Restarting...");
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
                        "mpsc loop: failed to register http handler /api/version: {:?}",
                        err
                    );
                    info!("mpsc loop: Restarting...");
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }

                if let Err(err) =
                    s.fn_handler("/api/ota", embedded_svc::http::Method::Post, move |req| {
                        // use embedded_svc::http::client::Connection
                        use embedded_svc::ota::{Ota, OtaUpdate};
                        use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
                        use esp_idf_svc::ota::EspOta;

                        let mut body: [u8; 1024 * 8];
                        let mut headers = Headers::<1>::new();
                        headers.set_cache_control("no-store");

                        let mut resp = req.into_response(200, None, headers.as_slice())?;

                        let res = req.connection().read(&mut body);
                        info!("POST body size: {}", res.unwrap());

                        let firmware = url::form_urlencoded::parse(&body)
                            .filter(|p| p.0 == "firmware")
                            .map(|p| p.1)
                            .next()
                            .ok_or_else(|| anyhow::anyhow!("No parameter firmware"));

                        info!("Will use firmware from: {:?}", firmware);

                        let mut ota = EspOta::new().unwrap();

                        let mut client = EspHttpConnection::new(&Configuration {
                            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
                            buffer_size_tx: Some(1024),
                            ..Default::default()
                        })?;

                        let mut resp = client.initiate_request(
                            embedded_svc::http::Method::Get,
                            &firmware.unwrap(),
                            &[("Content-Length", "0")],
                        );
                        client.initiate_response()?;

                        info!(">>>>>>>>>>>>>>>> initiating OTA update");

                        // let mut ota_update = ota.initiate_update().unwrap();
                        let mut firmware_update_ok = true;
                        let mut bytes_read = 0;

                        loop {
                            let bytes_to_take: usize = 8 * 1024;

                            let body: Result<Vec<u8>, _> = ToStd::new(response.reader())
                                .take(bytes_to_take.try_into().unwrap())
                                .bytes()
                                .collect();
                            let body = body?;
                            bytes_read += body.len();

                            info!(">>>>>>>>>>>>>> got new firmware batch {:?}", body.len());
                            if !body.is_empty() {
                                /*
                                match ota_update.write(&body) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        info!("failed to write update with: {:?}", err);
                                        firmware_update_ok = false;
                                        break;
                                    }
                                }
                                */
                            } else {
                                info!("!!!!! ERROR firmware image with zero length !!!!");
                            }

                            if body.len() < bytes_to_take {
                                break;
                            }

                            info!("Total firmware bytes read: {}", bytes_read);
                            sleep(Duration::from_millis(20));
                        }

                        info!(
                            ">>>>>>>>>>>>>>>> firmware update ok says: {:?}",
                            firmware_update_ok
                        );

                        if firmware_update_ok {
                            //  ota_update.complete().unwrap();
                            info!(">>>>>>>>>>>>>>>> completed firmware update");
                        } else {
                            // ota_update.abort().unwrap();
                        }
                        let _result = embedded_svc::httpd::Response::from("test").status;
                        resp.send_str(
                            r#"
                    <doctype html5>
                    <html>
                        <body>
                            Firmware updated. About to reboot now. Bye!
                        </body>
                    </html>
                    "#,
                        )?;

                        info!("Processing '/api/ota' request");
                        Ok(())
                    })
                {
                    info!(
                        "mpsc loop: failed to register http handler /api/ota: {:?}",
                        err
                    );
                    info!("mpsc loop: Restarting...");
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
                        "mpsc loop: failed to register http handler /temperature: {:?}",
                        err
                    );
                    info!("mpsc loop: Restarting...");
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }
            }
            Err(err) => {
                if err == mpsc::TryRecvError::Disconnected {
                    //reboot
                    info!("mpsc loop: error recv {:?}", err);
                    info!("mpsc loop: Restarting...");
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                } // the other error value is Empty which is okay and we ignore
            }
        }
        sleep(Duration::from_millis(100));
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
