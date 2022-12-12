use crate::gps_mtk3339::gps;
use crate::gps_mtk3339::gps::Mtk3339;
use crate::screen::anemometer_screen::LayoutManager;
use crate::web_server::url_handler;
use core::mem;
//use core::num::dec2flt::float;
//use core::str;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_svc::wifi::{self, AuthMethod, ClientConfiguration};
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::*;

use esp_idf_svc::http::server::Configuration;
use esp_idf_svc::timer::*;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    netif::IpEvent,
    nvs::EspDefaultNvsPartition,
    wifi::{EspWifi, WifiEvent, WifiWait},
};
use esp_idf_sys as _;
use esp_idf_sys::{self as sys, esp, esp_wifi_set_ps, wifi_ps_type_t_WIFI_PS_NONE};
use log::info;
use nmea;
use smart_leds::{colors::*, RGB8};
use std::format;
use std::net::Ipv4Addr;
use std::rc::Rc;
use std::str;
use std::sync::mpsc;
use std::{thread::sleep, time::Duration};
use sys::EspError;
mod display;
mod errors;
mod gps_mtk3339;
mod lazy_http_server;
mod neopixel;
mod peripherals;
mod screen;
mod sdmmc;
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

enum WidgetName {
    GpsSpeed,
    WindSpeed,
}

struct DisplayCmd {
    widget: WidgetName,
    text: String,
}

enum SysLoopMsg {
    WifiDisconnect,
    IpAddressAsquired { ip: Ipv4Addr },
    NeopixelMsg { color: RGB8 },
    DisplayMsg { cmd: DisplayCmd },
    OtaUpdateStarted,
    NmeaData { data: String },
}

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = peripherals::SystemPeripherals::take();

    let mut status_led = neopixel::ws2812::NeoPixel::new(peripherals.neopixel)?;
    status_led.power_on(true);

    status_led.write(DARK_ORANGE)?;

    let display_peripherals = peripherals.display;
    let spi_bus_driver = peripherals.spi_bus.driver;
    let sdmmc_peripherals = peripherals.sdcard;

    let mut sd_card =
        sdmmc::sd_storage::SdCard::new(sdmmc_peripherals, Rc::clone(&spi_bus_driver))?;

    let mut display = display::display(display_peripherals, Rc::clone(&spi_bus_driver)).unwrap();
    let backlight = peripherals.display_backlight;

    display.clear(Rgb565::BLACK).unwrap();

    let layout_mgr = LayoutManager::new()?;

    layout_mgr.draw_initial_screen(&mut display).unwrap();
    layout_mgr
        .draw_sw_version(&mut display, format!("FW: V{}", FIRMWARE_VERSION).as_str())
        .unwrap();

    // we do it here to prevent garbage on the screen
    turn_backlight_on(backlight);

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

    let tx0 = tx.clone();
    let tx1 = tx.clone();
    let tx2 = tx.clone();
    let tx3 = tx.clone();
    //let tx4 = tx.clone();
    let tx5 = tx.clone();
    //let tx6 = tx.clone();

    info!(" ************** Before UART backgound thread started");
    let _ = std::thread::Builder::new()
        .stack_size(16_000)
        .spawn(move || {
            info!("GPS Listening for messages");
            print_stack_remaining_size(16_000);
            let mut nmea = nmea::Nmea::default();
            print_stack_remaining_size(16_000);

            info!("Size of NMEA: {}", std::mem::size_of::<nmea::Nmea>());

            info!("Configure GPS receiver");
            let mut gps = Mtk3339::new(
                9600,
                peripherals.gps.uart1,
                peripherals.gps.tx,
                peripherals.gps.rx,
            )
            .unwrap();

            gps.send_command(gps::PMTK_SET_NMEA_OUTPUT_RMCGGA);
            gps.send_command(gps::PMTK_GPS_GLONASS);

            loop {
                let mut s = gps.read_line().unwrap();
                s = gps::Mtk3339::fix_rmc_sentence(s);

                info!("NMEA len:{} raw: {:?}", s.len(), s);

                if s.len() > 0 {
                    tx.send(SysLoopMsg::NmeaData {
                        data: format!("{}\n", s),
                    })
                    .unwrap();
                    info!("================= NMEA parse");
                    let res = nmea.parse(s.as_str());

                    match res {
                        Ok(_res) => {
                            info!(
                                "NMEA latetude: {:.6}, longitude: {:.6}",
                                if nmea.latitude.is_some() {
                                    nmea.latitude.unwrap() as f32
                                } else {
                                    0.
                                },
                                if nmea.longitude.is_some() {
                                    nmea.longitude.unwrap() as f32
                                } else {
                                    0.
                                }
                            );
                            let speed = if nmea.speed_over_ground.is_some() {
                                (nmea.speed_over_ground.unwrap() as f32) * 1.852_f32
                            } else {
                                0.
                            };
                            info!("NMEA speed: {:.1} km/h", speed);

                            tx.send(SysLoopMsg::DisplayMsg {
                                cmd: DisplayCmd {
                                    widget: WidgetName::GpsSpeed,
                                    text: format!("GPS: {:4.1}", speed),
                                },
                            })
                            .unwrap();
                        }
                        Err(e) => info!("******* NEMEA error : {e:?} *******"),
                    }
                }
                esp_idf_hal::delay::FreeRtos::delay_ms(1000);
            }
        })
        .unwrap();

    info!(" ************** After UART backgound thread started");

    //let timer1 = gps_signal_sim_timer(tx4).unwrap();
    let timer2 = wind_signal_sim_timer(tx5).unwrap();

    let _wifi_event_sub = sysloop.subscribe(move |event: &WifiEvent| match event {
        WifiEvent::StaConnected => {
            info!("******* Received STA Connected Event");
        }
        WifiEvent::StaDisconnected => {
            info!("******* Received STA Disconnected event");
            tx0.send(SysLoopMsg::WifiDisconnect)
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
            Ok(SysLoopMsg::DisplayMsg { cmd }) => {
                match cmd.widget {
                    WidgetName::GpsSpeed => {
                        layout_mgr
                            .draw_gps_speed(&mut display, &cmd.text.as_str())
                            .unwrap();
                    }
                    WidgetName::WindSpeed => {
                        layout_mgr
                            .draw_wind_speed(&mut display, &cmd.text.as_str())
                            .unwrap();
                    }
                };
            }
            Ok(SysLoopMsg::NmeaData { data }) => {
                sd_card.write(data);
            }
            Ok(SysLoopMsg::NeopixelMsg { color }) => {
                status_led.write(color)?;
            }
            Ok(SysLoopMsg::OtaUpdateStarted) => {
                info!("OTA Update started - stopping timer and IRQ");
                //timer1.cancel().unwrap();
                timer2.cancel().unwrap();
            }
            Ok(SysLoopMsg::WifiDisconnect) => {
                info!("mpsc loop: WifiDisconnect received");

                httpd.clear();
                tx2.send(SysLoopMsg::NeopixelMsg { color: RED })?;
                layout_mgr.draw_ip_address(&mut display, " ").unwrap();
            }
            Ok(SysLoopMsg::IpAddressAsquired { ip }) => {
                info!("mpsc loop: IpAddressAsquired received");
                let tx4 = tx3.clone();

                tx3.send(SysLoopMsg::NeopixelMsg { color: DARK_GREEN })?;

                layout_mgr
                    .draw_ip_address(&mut display, format!("IP: {}", ip.to_string()).as_str())
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
                        tx4.send(SysLoopMsg::OtaUpdateStarted)?;
                        esp_idf_hal::delay::FreeRtos::delay_ms(100);
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

fn turn_backlight_on(p: AnyOutputPin) {
    let mut backlight = PinDriver::output(p).unwrap();

    backlight.set_drive_strength(DriveStrength::I40mA).unwrap();
    backlight.set_high().unwrap();

    mem::forget(backlight); // TODO: For now
}

#[allow(dead_code)]
fn gps_signal_sim_timer(tx: std::sync::mpsc::Sender<SysLoopMsg>) -> Result<EspTimer, EspError> {
    let periodic_timer = EspTimerService::new()?.timer(move || {
        let random_number = unsafe { esp_idf_sys::esp_random() };
        let sim_gps_speed: u32 = random_number % (33 - 15 + 1) + 15;

        tx.send(SysLoopMsg::DisplayMsg {
            cmd: DisplayCmd {
                widget: WidgetName::GpsSpeed,
                text: format!("GPS: {}", sim_gps_speed),
            },
        })
        .unwrap();
    })?;

    periodic_timer.every(Duration::from_secs(5))?;

    Ok(periodic_timer)
}

fn wind_signal_sim_timer(tx: std::sync::mpsc::Sender<SysLoopMsg>) -> Result<EspTimer, EspError> {
    let periodic_timer = EspTimerService::new()?.timer(move || {
        let random_number = unsafe { esp_idf_sys::esp_random() };
        let sim_wind_speed: u32 = random_number % (33 - 15 + 1) + 15;

        tx.send(SysLoopMsg::DisplayMsg {
            cmd: DisplayCmd {
                widget: WidgetName::WindSpeed,
                text: format!("Win: {}", sim_wind_speed),
            },
        })
        .unwrap();
    })?;

    periodic_timer.every(Duration::from_secs(6))?;

    Ok(periodic_timer)
}

fn print_stack_remaining_size(stack_size: u32) {
    let stack = unsafe { esp_idf_sys::uxTaskGetStackHighWaterMark(core::ptr::null_mut()) };
    let left = stack_size - stack;
    info!("stack use high water mark {left}/{stack_size}");
}
