/*
 * ESP32 Anemometer
 *
 * MIT license
 *
 * Copyright (c) 2021-2023 Michael Zill
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 * Apache license, Version 2.0
 *
 * Copyright (c) 2021-2023 Michael Zill
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::configuration::AwsIoTCertificates;
use crate::mqtt_msg::*;
use crate::utils::errors::*;
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

pub fn mqtt(
    aws_certificates: &'static AwsIoTCertificates,
) -> Result<
    (
        impl Client + Publish,
        impl Connection<Message = Option<MqttCommand>>,
    ),
    InitError,
> {
    info!("Enter mqtt");
    let mut mqtt_parser = MessageParser::new();

    let x509_client_cert = esp_idf_svc::tls::X509::pem_until_nul(&aws_certificates.device_cert[..]);
    let x509_client_priv_key =
        esp_idf_svc::tls::X509::pem_until_nul(&aws_certificates.private_key[..]);

    // need to remove tailing zeros otherwise CString will complain
    let url = core::str::from_utf8(
        &(aws_certificates.mqtt_endpoint[0..aws_certificates
            .mqtt_endpoint
            .iter()
            .position(|&x| x == 0)
            .unwrap()]),
    )
    .unwrap();
    info!("url = {url}");

    let device_id = core::str::from_utf8(
        &(aws_certificates.device_id[0..aws_certificates
            .device_id
            .iter()
            .position(|&x| x == 0)
            .unwrap()]),
    )
    .unwrap();
    info!("client id = {device_id}");

    let (mqtt_client, mqtt_conn) = EspMqttClient::new_with_converting_async_conn(
        url,
        &MqttClientConfiguration {
            client_id: Some(device_id),
            client_certificate: Some(x509_client_cert),
            private_key: Some(x509_client_priv_key),
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            ..Default::default()
        },
        move |event| mqtt_parser.convert(event),
    )?;
    let mqtt_client = mqtt_client.into_async();

    Ok((mqtt_client, mqtt_conn))
}

#[allow(dead_code)]
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

pub mod anemometer {

    // This value is incremented by the ISR
    static ANEMOMETER_PULSCOUNT: AtomicU32 = AtomicU32::new(0);
    use crate::global_settings;
    use crate::state::*;
    use crate::utils::errors::*;
    use esp_idf_hal::gpio::*;
    use esp_idf_hal::peripheral::Peripheral;
    use esp_idf_svc::timer::*;
    use esp_idf_sys::*;
    use fixed::{types::extra::U4, FixedU16};
    use std::sync::atomic::*;
    use std::time::Duration;

    pub struct AnemometerDriver<P>
    where
        P: Pin,
    {
        _pin: PinDriver<'static, P, Input>,
    }

    impl<P: InputPin + OutputPin> AnemometerDriver<P> {
        pub fn new(
            pin: impl Peripheral<P = P> + 'static,
        ) -> Result<AnemometerDriver<P>, InitError> {
            Ok(AnemometerDriver {
                _pin: subscribe_pin(pin, count_pulse)?,
            })
        }

        // This timer reads at a defined frequence the counter for rotation
        // pulses (incremented by the ISR) and stores the values in the
        // wind historian to calculating averages which gets send via
        // MQTT messages.
        pub fn set_measurement_timer(&mut self) -> Result<EspTimer, EspError> {
            let periodic_timer = EspTimerService::new()?.timer(move || {
                // load puls count and set to zero
                let cnt = ANEMOMETER_PULSCOUNT.fetch_and(0, Ordering::Relaxed);

                // We receive 2 pulses per rotatio, therefor the counter needs
                // to be devided by 2. MEASUREMENT_INTERVAL needs to be in [ms]
                #[allow(unused_variables)]
                let rps = (FixedU16::<U4>::from_num(cnt >> 2)
                    / (FixedU16::<U4>::from_num(global_settings::MEASUREMENT_INTERVAL as u16)
                        / FixedU16::<U4>::from_num(1000)))
                .to_num::<u16>();

                // let rps = cnt as f32 / 2.0 / (MEASUREMENT_INTERVAL as u32) as f32;

                // TODO: Remove once anemometer is connected
                let rps = (unsafe { esp_random() } % 0xf) as u16;
                let mut wind_historian = (*WIND_DATA_HISTORY).lock().unwrap();
                wind_historian.store_measurement(rps, 0);
            })?;

            periodic_timer.every(Duration::from_millis(global_settings::MEASUREMENT_INTERVAL))?;

            Ok(periodic_timer)
        }
    }

    fn count_pulse() {
        ANEMOMETER_PULSCOUNT.fetch_add(1, Ordering::Relaxed);
    }

    fn subscribe_pin<'d, P: InputPin + OutputPin>(
        pin: impl Peripheral<P = P> + 'd,
        notify: impl Fn() + 'static,
    ) -> Result<PinDriver<'d, P, Input>, InitError> {
        let mut pin = PinDriver::input(pin)?;

        // in case the input pin is not connected to any ciruit
        //pin.set_pull(Pull::Down)?;
        pin.set_interrupt_type(InterruptType::NegEdge)?;

        unsafe {
            pin.subscribe(notify)?;
        }
        Ok(pin)
    }
}
