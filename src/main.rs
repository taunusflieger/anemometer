use crate::web_server::url_handler;
use core::str;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
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
use std::format;
use std::net::Ipv4Addr;
use std::sync::mpsc;
use std::{thread::sleep, time::Duration};
use u8g2_fonts::types::HorizontalAlignment;
use u8g2_fonts::{
    fonts,
    types::{FontColor, VerticalPosition},
    FontRenderer,
};
mod errors;
mod lazy_http_server;
mod neopixel;
mod peripherals;

mod services;
mod web_server;

sys::esp_app_desc!();

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
    IpAddressAsquired { ip: Ipv4Addr },
    NeopixelMsg { color: RGB8 },
}

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = peripherals::SystemPeripherals::take();

    let mut status_led = neopixel::ws2812::NeoPixel::new(peripherals.neopixel)?;

    status_led.write(DARK_ORANGE)?;

    // ******************************************************************
    // Experimental
    // ******************************************************************
    #[cfg(feature = "tft")]
    let display_peripherals = peripherals.display;
    let mut display = services::display(display_peripherals).unwrap();

    let _d = display.clear(Rgb565::BLACK);

    let font = FontRenderer::new::<fonts::u8g2_font_logisoso16_tf>();

    let text = "ESP32-S3 Anemometer";

    font.render_aligned(
        text,
        display.bounding_box().center() - Point::new(115, 35),
        VerticalPosition::Baseline,
        HorizontalAlignment::Left,
        FontColor::Transparent(Rgb565::RED),
        &mut display,
    )
    .unwrap();

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
            tx.send(SysLoopMsg::WifiDisconnect)
                .expect("wifi event channel closed");
            if let Err(err) = wifi.connect() {
                info!("Error calling wifi.connect in wifi reconnect {:?}", err);
            }
        }
        _ => info!("Received other Wifi event"),
    })?;

    let _ip_event_sub = sysloop.subscribe(move |event: &IpEvent| match event {
        IpEvent::DhcpIpAssigned(assignment) => {
            info!(
                "************ Received IPEvent address assigned {:?}",
                assignment.ip_settings.ip
            );

            tx1.send(SysLoopMsg::IpAddressAsquired {
                ip: assignment.ip_settings.ip,
            })
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
            Ok(SysLoopMsg::IpAddressAsquired { ip }) => {
                info!("mpsc loop: IpAddressAsquired received");
                let tx4 = tx3.clone();

                tx3.send(SysLoopMsg::NeopixelMsg { color: DARK_GREEN })?;

                let text = format!("IP: {}  FW: v{}", ip.to_string(), FIRMWARE_VERSION);
                //"IP: 192.168.100.102  FW: v0.38.21";
                let font = FontRenderer::new::<fonts::u8g2_font_t0_14_tf>();
                font.render_aligned(
                    text.as_str(),
                    display.bounding_box().center() - Point::new(115, -60),
                    VerticalPosition::Baseline,
                    HorizontalAlignment::Left,
                    FontColor::Transparent(Rgb565::WHITE),
                    &mut display,
                )
                .unwrap();
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
