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
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
use crate::configuration::AwsIoTSettings;
use crate::global_settings::*;
use crate::services::*;
use crate::state::*;
use crate::task::{httpd, mqtt, ota::*, publisher};
use crate::utils::nvs_ext::*;
use crate::utils::{datetime, errors::*};
use channel_bridge::{asynch::pubsub, asynch::*};
use configuration::AwsIoTCertificates;
use edge_executor::*;
use edge_executor::{Local, Task};
use embedded_svc::utils::asyncify::Asyncify;
use embedded_svc::wifi::Wifi as WifiTrait;
use esp_idf_hal::reset::WakeupReason;
use esp_idf_hal::task::thread::ThreadSpawnConfiguration;
use esp_idf_hal::{cpu::*, task::executor::EspExecutor};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::netif::IpEvent;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::nvs::*;
use esp_idf_svc::wifi::WifiEvent;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use esp_idf_sys::esp_ota_mark_app_valid_cancel_rollback;
use esp_idf_sys::{self as sys, esp, esp_wifi_set_ps, wifi_ps_type_t_WIFI_PS_MIN_MODEM};
use log::*;
use once_cell::sync::Lazy;
use std::sync::Mutex;

mod configuration;
mod data_processing;
mod global_settings;
mod mqtt_msg;
mod peripherals;
mod services;
mod state;
mod task;
mod utils;

//sys::esp_app_desc!();

static AWSCERTIFICATES: static_cell::StaticCell<AwsIoTCertificates> =
    static_cell::StaticCell::new();

pub static AWSCONFIG: Lazy<Mutex<AwsIoTSettings>> = Lazy::new(|| {
    Mutex::new(match AwsIoTSettings::new("conf") {
        Ok(settings) => settings,
        Err(err) => {
            error!("Failed to load AWS configuration: {err}");
            panic!();
        }
    })
});

fn main() -> core::result::Result<(), InitError> {
    esp_idf_hal::task::critical_section::link();
    esp_idf_svc::timer::embassy_time::driver::link();
    esp_idf_svc::timer::embassy_time::queue::link();

    esp_idf_svc::log::EspLogger::initialize_default();

    info!("ESP32-Anemometer");
    match core() {
        Core::Core0 => info!("running on core 0"),
        Core::Core1 => info!("running on core 1"),
    }
    let wakeup_reason = WakeupReason::get();
    info!("Wakeup reason: {:?}", wakeup_reason);

    let pm_config = esp_idf_sys::esp_pm_config_esp32s3_t {
        max_freq_mhz: MAX_CPU_FREQ,
        min_freq_mhz: MIN_CPU_FREQ,
        light_sleep_enable: LIGHT_SLEEP_MODE_ENABLED,
    };

    if let Err(err) =
        esp!(unsafe { esp_idf_sys::esp_pm_configure(&pm_config as *const _ as *const _) })
    {
        panic!("failed to set esp_pm_configure {err}");
    }

    let peripherals = peripherals::SystemPeripherals::take();
    let anemometer_peripherals = peripherals.pulse_counter;
    let nvs_default_partition = EspDefaultNvsPartition::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    // Initialize data capture from anemometer
    let mut anemometer = anemometer::AnemometerDriver::new(anemometer_peripherals.pulse).unwrap();

    let _anemometer_timer = anemometer.set_measurement_timer().unwrap();

    let aws_iot_certificates: &'static AwsIoTCertificates =
        AWSCERTIFICATES.init(match AwsIoTCertificates::new("conf") {
            Ok(settings) => settings,
            Err(err) => {
                error!("Failed to load AWS configuration: {err}");
                panic!();
            }
        });

    let (wifi, wifi_notif) = wifi(
        peripherals.modem,
        sysloop.clone(),
        Some(nvs_default_partition),
    )?;

    esp!(unsafe { esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_MIN_MODEM) })?;

    let _sntp = utils::datetime::initialize();

    ThreadSpawnConfiguration {
        name: Some(b"high-prio-executor\0"),
        priority: TASK_HIGH_PRIORITY,
        ..Default::default()
    }
    .set()?;

    let _high_prio_execution = schedule::<8, _>(40000, move || {
        let executor = EspExecutor::new();
        let mut tasks = heapless::Vec::new();

        executor.spawn_local_collect(ota_task(aws_iot_certificates), &mut tasks)?;
        executor.spawn_local_collect(process_wifi_state_change(wifi, wifi_notif), &mut tasks)?;

        executor.spawn_local_collect(
            process_netif_state_change(netif_notifier(sysloop.clone()).unwrap()),
            &mut tasks,
        )?;

        Ok((executor, tasks))
    });

    ThreadSpawnConfiguration {
        name: Some(b"mid-prio-executor\0"),
        priority: TASK_MID_PRIORITY,
        ..Default::default()
    }
    .set()?;

    std::thread::sleep(core::time::Duration::from_millis(8000));

    let _mid_prio_execution = schedule::<8, _>(8000, move || {
        let executor = EspExecutor::new();
        let mut tasks = heapless::Vec::new();
        let (mqtt_client, mqtt_conn) = services::mqtt(aws_iot_certificates).unwrap();

        executor.spawn_local_collect(
            mqtt::send_task::<MQTT_MAX_TOPIC_LEN>(mqtt_client),
            &mut tasks,
        )?;
        executor.spawn_local_collect(mqtt::receive_task(mqtt_conn), &mut tasks)?;
        Ok((executor, tasks))
    });

    ThreadSpawnConfiguration {
        name: Some(b"low-prio-executor\0"),
        priority: TASK_LOW_PRIORITY,
        ..Default::default()
    }
    .set()?;

    let low_prio_execution = schedule::<8, _>(32000, move || {
        let executor = EspExecutor::new();
        let mut tasks = heapless::Vec::new();
        executor.spawn_local_collect(publisher::wind_speed_task(), &mut tasks)?;
        //executor.spawn_local_collect(httpd::http_server_task(), &mut tasks)?;

        Ok((executor, tasks))
    });

    if let Ok(datetime) = datetime::get_datetime() {
        let format =
            time::format_description::parse("[day].[month].[year] [hour]:[minute]:[second]")
                .expect("Invalid format.");

        let time = datetime.format(&format).expect("Could not format time.");
        info!("System start time: {time}");
    }

    low_prio_execution.join().unwrap();

    unreachable!();
}

pub fn schedule<'a, const C: usize, M>(
    stack_size: usize,
    spawner: impl FnOnce() -> core::result::Result<
            (Executor<'a, C, M, Local>, heapless::Vec<Task<()>, C>),
            SpawnError,
        > + Send
        + 'static,
) -> std::thread::JoinHandle<()>
where
    M: Monitor + Wait + Default,
{
    std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(move || {
            let (executor, tasks) = spawner().unwrap();

            executor.run_tasks(|| true, tasks);
        })
        .unwrap()
}

#[inline(always)]
pub fn netif_notifier(
    mut sysloop: EspSystemEventLoop,
) -> core::result::Result<impl Receiver<Data = IpEvent>, InitError> {
    Ok(pubsub::SvcReceiver::new(sysloop.as_async().subscribe()?))
}

pub async fn process_wifi_state_change(
    mut wifi: impl WifiTrait,
    mut state_changed_source: impl Receiver<Data = WifiEvent>,
) {
    loop {
        let event = state_changed_source.recv().await.unwrap();

        match event {
            WifiEvent::StaConnected => {}
            WifiEvent::StaDisconnected => {
                let mut publisher = NETWORK_EVENT_CHANNEL.publisher().unwrap();
                let _ = publisher.send(NetworkStateChange::WifiDisconnected).await;
                let _ = wifi.connect();
            }
            _ => {}
        }
    }
}

pub async fn process_netif_state_change(mut state_changed_source: impl Receiver<Data = IpEvent>) {
    loop {
        if let IpEvent::DhcpIpAssigned(assignment) = state_changed_source.recv().await.unwrap() {
            info!("IpEvent: DhcpIpAssigned: {:?}", assignment.ip_settings.ip);

            // if an IP address has been succesfully assiggned we consider
            // the application working, no rollback required.
            unsafe { esp_ota_mark_app_valid_cancel_rollback() };

            let mut publisher = NETWORK_EVENT_CHANNEL.publisher().unwrap();
            let _ = publisher
                .send(NetworkStateChange::IpAddressAssigned {
                    ip: assignment.ip_settings.ip,
                })
                .await;
        }
    }
}
