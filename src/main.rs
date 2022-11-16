use core::str;
use embassy_sync::blocking_mutex::Mutex;
use embedded_svc::wifi::{self, AuthMethod, ClientConfiguration};
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::{FixedLengthSignal, PinState, Pulse, RmtChannel, TxRmtDriver};
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

//static MY_GPIO: Mutex<RefCell<Option<esp_idf_hal::gpio::Gpio18>>> = Mutex::new(RefCell::new(None));
type LedPin = esp_idf_hal::gpio::PinDriver<'static, Gpio18, Output>;

static NEOPIXEL_PIN: static_cell::StaticCell<Arc<Mutex<EspRawMutex, RefCell<LedPin>>>> =
    static_cell::StaticCell::new();

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Required for neopixel
    let peripherals = Peripherals::take().unwrap();

    /*
        let a = MY_GPIO
            .lock()
            .unwrap()
            .borrow_mut()
            .replace(test { a: tc, b: 34 });
    }); */

    let mut channel = peripherals.rmt.channel0;
    let config = TransmitConfig::new().clock_divider(1);

    // Required for UM TinyS3 board. The WS2812 VDD pin is connected
    // to PIN 17, so it needs to be powered through the PIN
    let led_pwr = peripherals.pins.gpio17;
    let mut led_pwr = PinDriver::output(led_pwr)?;
    led_pwr.set_high()?;

    let led_pin_handle = NEOPIXEL_PIN.init(Arc::new(Mutex::new(RefCell::new(
        PinDriver::output(peripherals.pins.gpio18).unwrap(),
    ))));

    //neopixel2(NeopixelColor::Red, &mut channel, &config)?;

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

                //neopixel2(NeopixelColor::Red, &mut channel, &config)?;
                httpd.clear();
            }
            Ok(SysLoopMsg::IpAddressAsquired) => {
                info!("mpsc loop: IpAddressAsquired received");

                //neopixel2(NeopixelColor::Green, &mut channel, &config)?;
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
    channel: impl Peripheral<P = impl RmtChannel>,
    config: &TransmitConfig,
    led: impl Peripheral<P = impl OutputPin>,
) -> anyhow::Result<()> {
    let mut tx = TxRmtDriver::new(channel, led, &config)?;

    let ticks_hz = tx.counter_clock()?;
    let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350))?;
    let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800))?;
    let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700))?;
    let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600))?;

    let mut signal = FixedLengthSignal::<24>::new();
    for i in 0..24 {
        let bit = 2_u32.pow(i) & color as u32 != 0;
        let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
        signal.set(i as usize, &(high_pulse, low_pulse))?;
    }
    tx.start_blocking(&signal)?;

    Ok(())
}
/*
fn neopixel2(
    color: NeopixelColor,
    channel: impl Peripheral<P = impl RmtChannel>,
    config: &TransmitConfig,
    led_pin: Arc<Mutex<EspRawMutex, RefCell<LedPin>>>,
) -> anyhow::Result<()> {
    led_pin.lock(|led_pin| {
        let mut led = led_pin.borrow_mut();
        let mut tx = TxRmtDriver::new(channel, led, &config)?;

        let ticks_hz = tx.counter_clock()?;
        let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350))?;
        let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800))?;
        let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700))?;
        let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600))?;

        let mut signal = FixedLengthSignal::<24>::new();
        for i in 0..24 {
            let bit = 2_u32.pow(i) & color as u32 != 0;
            let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
            signal.set(i as usize, &(high_pulse, low_pulse))?;
        }
        tx.start_blocking(&signal)?;
    });
    Ok(())
}
*/
