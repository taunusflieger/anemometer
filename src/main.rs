use crate::web_server::url_handler;
use core::str;
use embedded_svc::wifi::{self, AuthMethod, ClientConfiguration};
use esp_idf_hal::gpio::*;
use esp_idf_svc::http::server::Configuration;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    netif::IpEvent,
    nvs::EspDefaultNvsPartition,
    wifi::{EspWifi, WifiEvent, WifiWait},
};
use esp_idf_sys as _;
use esp_idf_sys::{self as sys, esp, esp_wifi_set_ps, wifi_ps_type_t_WIFI_PS_NONE};
use log::info;

use smart_leds::{colors::*, RGB8};
use std::{thread::sleep, time::Duration};

use std::sync::mpsc;
mod errors;
mod lazy_http_server;
mod neopixel;
mod peripherals;

mod services;
mod web_server;

sys::esp_app_desc!();

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
    NeopixelMsg { color: RGB8 },
}

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = peripherals::SystemPeripherals::take();

    let mut status_led =
        neopixel::ws2812::NeoPixel::new(peripherals.neopixel.channel, peripherals.neopixel.pin)?;

    // Required for neopixel
    let mut pwr_pin = PinDriver::output(peripherals.neopixel.dc)?;
    pwr_pin.set_high()?;

    status_led.write(DARK_ORANGE)?;

    // ******************************************************************
    // Need to set reset pin for display here
    // TODO: find a way to move this into services.rs
    // ******************************************************************
    #[cfg(feature = "tft")]
    let rst_pin = PinDriver::output(peripherals.display_rst)?;

    // ******************************************************************
    // ******************************************************************
    let httpd = lazy_http_server::lazy_init_http_server::LazyInitHttpServer::new();
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
    let tx2 = tx.clone();
    let tx3 = tx.clone();

    let _wifi_event_sub = sysloop.subscribe(move |event: &WifiEvent| match event {
        WifiEvent::StaConnected => {
            info!("******* Received STA Connected Event");
        }
        WifiEvent::StaDisconnected => {
            info!("******* Received STA Disconnected event");
            /*
            if let Err(err) = neopixel(NeopixelColor::Red, neopixel_ctx1.clone()) {
                info!("Error using neopixel {:?}", err);
            } */
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
            /*
            if let Err(err) = neopixel(NeopixelColor::Green, neopixel_ctx2.clone()) {
                info!("Error using neopixel {:?}", err);
            } */
            tx1.send(SysLoopMsg::IpAddressAsquired)
                .expect("IP event channel closed");
        }
        _ => info!("Received other IPEvent"),
    })?;

    loop {
        match rx.try_recv() {
            Ok(SysLoopMsg::NeopixelMsg { color }) => {
                status_led.write(color)?;
            }
            Ok(SysLoopMsg::WifiDisconnect) => {
                info!("mpsc loop: WifiDisconnect received");

                httpd.clear();
                tx2.send(SysLoopMsg::NeopixelMsg { color: RED })?;
            }
            Ok(SysLoopMsg::IpAddressAsquired) => {
                info!("mpsc loop: IpAddressAsquired received");
                let tx4 = tx3.clone();

                tx3.send(SysLoopMsg::NeopixelMsg { color: DARK_GREEN })?;
                let server_config = Configuration::default();
                let mut s = httpd.create(&server_config);

                if let Err(err) = s.fn_handler("/", embedded_svc::http::Method::Get, move |req| {
                    url_handler::home_page_handler(req)
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
                    move |req| url_handler::api_version_handler(req),
                ) {
                    info!(
                        "mpsc loop: failed to register http handler /api/version: {:?} - restarting device",
                        err
                    );
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }

                if let Err(err) =
                    s.fn_handler("/api/ota", embedded_svc::http::Method::Post, move |req| {
                        tx4.send(SysLoopMsg::NeopixelMsg { color: BLUE })?;
                        url_handler::ota_update_handler(req)
                    })
                {
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
                    move |req| url_handler::temperature_handler(req),
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
