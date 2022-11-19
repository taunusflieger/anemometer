use core::str;
use embassy_sync::blocking_mutex::Mutex;
use embedded_svc::wifi::{self, AuthMethod, ClientConfiguration};
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::{FixedLengthSignal, PinState, Pulse, TxRmtDriver};
use esp_idf_hal::task::embassy_sync::EspRawMutex;
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
use std::{thread::sleep, time::Duration};

use std::cell::RefCell;
use std::sync::{mpsc, Arc};

use crate::web_server::url_handler;
mod lazy_http_server;
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
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum NeopixelColor {
    Blue = 0xff0000,
    Red = 0x00ff00,
    Green = 0x0000ff,
}

struct NeoPixelContext {
    channel: RefCell<esp_idf_hal::rmt::CHANNEL0>,
    tx_config: esp_idf_hal::rmt::RmtTransmitConfig,
    // For UM TinyS3 board Gpio18
    // For Adafruit ESP32-S3 Gpio33
    pin: RefCell<esp_idf_hal::gpio::Gpio33>,
}

static NEOPIXEL_CTX: static_cell::StaticCell<Arc<Mutex<EspRawMutex, RefCell<NeoPixelContext>>>> =
    static_cell::StaticCell::new();

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Required for neopixel
    let peripherals = Peripherals::take().unwrap();

    // Required for UM TinyS3 board. The WS2812 VDD pin is connected
    // to PIN 17, so it needs to be powered through the PIN
    // Required for Adafruit Feather ESP32-S3. The WS2812 VDD pin is connected
    // to PIN 21, so it needs to be powered through the PIN

    let led_pwr = peripherals.pins.gpio21;
    let mut led_pwr = PinDriver::output(led_pwr)?;
    led_pwr.set_high()?;

    let nexpixel_ctx_handle =
        NEOPIXEL_CTX.init(Arc::new(Mutex::new(RefCell::new(NeoPixelContext {
            pin: RefCell::new(peripherals.pins.gpio33),
            channel: RefCell::new(peripherals.rmt.channel0),
            tx_config: TransmitConfig::new().clock_divider(1),
        }))));
    let neopixel_ctx0 = nexpixel_ctx_handle.clone();
    let neopixel_ctx1 = nexpixel_ctx_handle.clone();
    let neopixel_ctx2 = nexpixel_ctx_handle.clone();

    neopixel(NeopixelColor::Red, neopixel_ctx0)?;

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
    let _wifi_event_sub = sysloop.subscribe(move |event: &WifiEvent| match event {
        WifiEvent::StaConnected => {
            info!("******* Received STA Connected Event");
        }
        WifiEvent::StaDisconnected => {
            info!("******* Received STA Disconnected event");
            if let Err(err) = neopixel(NeopixelColor::Red, neopixel_ctx1.clone()) {
                info!("Error using neopixel {:?}", err);
            }
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
            if let Err(err) = neopixel(NeopixelColor::Green, neopixel_ctx2.clone()) {
                info!("Error using neopixel {:?}", err);
            }
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

fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

fn neopixel(
    color: NeopixelColor,
    ctx: Arc<Mutex<EspRawMutex, RefCell<NeoPixelContext>>>,
) -> anyhow::Result<()> {
    ctx.lock(|ctx| {
        let ctx = ctx.borrow_mut();

        let mut tx = TxRmtDriver::new(
            ctx.channel.borrow_mut(),
            ctx.pin.borrow_mut(),
            &ctx.tx_config,
        )
        .unwrap();

        let ticks_hz = tx.counter_clock().unwrap();
        let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350)).unwrap();
        let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800)).unwrap();
        let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700)).unwrap();
        let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600)).unwrap();

        let mut signal = FixedLengthSignal::<24>::new();
        for i in 0..24 {
            let bit = 2_u32.pow(i) & color as u32 != 0;
            let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
            signal.set(i as usize, &(high_pulse, low_pulse)).unwrap();
        }
        tx.start_blocking(&signal).unwrap();
    });
    Ok(())
}
