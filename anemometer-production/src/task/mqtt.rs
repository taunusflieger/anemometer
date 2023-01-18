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

use crate::data_processing::*;
use crate::mqtt_msg::{AWSShadowUpdate, MqttCommand};
use crate::state::*;
use crate::utils::datetime;
use crate::utils::error;
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embedded_svc::mqtt::client::asynch::{Client, Connection, Event, Publish, QoS};
use log::*;
use std::time::SystemTime;

static MQTT_CONNECT_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

pub async fn receive_task(mut connection: impl Connection<Message = Option<MqttCommand>>) {
    let mut app_event = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();

    loop {
        let (message, app_state_change) =
            match select(connection.next(), app_event.next_message_pure()).await {
                Either::First(message) => {
                    //info!("receive_task recv MQTT_CONNECT_SIGNAL");
                    (message, None)
                }
                Either::Second(app_state_change) => {
                    info!("receive_task recv app_state_change");
                    (None, Some(app_state_change))
                }
            };

        if let Some(message) = message {
            info!("receive_task [MQTT/CONNECTION]: {:?}", message);

            if let Ok(Event::Received(Some(cmd))) = &message {
                match cmd {
                    MqttCommand::ExecOTAUpdate(url) => {
                        info!(
                            "receive_task MQTT received OTA update request. url = {}",
                            url
                        );
                        let publisher = APPLICATION_EVENT_CHANNEL.publisher().unwrap();
                        let data = ApplicationStateChange::OTAUpdateRequest(url.clone());
                        publisher.publish(data).await;
                    }
                    MqttCommand::SystemRestart => {
                        info!("receive_task MQTT received system restart request");
                        unsafe {
                            esp_idf_sys::esp_restart();
                        }
                    }
                }
            } else if matches!(&message, Ok(Event::Connected(_))) {
                MQTT_CONNECT_SIGNAL.signal(true);
            } else if matches!(&message, Ok(Event::Disconnected)) {
                MQTT_CONNECT_SIGNAL.signal(false);
            }
        }

        if let Some(ApplicationStateChange::OTAUpdateStarted) = app_state_change {
            info!("receive_task OTA Update started shutting down mqtt receive_task");
            // No clean-up of the mqtt object here as this has been done in
            // send_task
            break;
        }
    }
}

// send will react on application state change event and then send the MQTT message
// the application state change event will be fired if new wind data is availbale.
// the requence in which MQTT messages are send depends on how often the application
// state change events gets fired.
// we are not implementing explicit re-connect logic, as this is already implemented
// in ESP IDF for MQTT.
pub async fn send_task<const L: usize>(mut mqtt: impl Client + Publish) {
    let mut connected = false;
    info!("Send Task Started");
    /*
        let topic = |topic_suffix| {
            heapless::String::<L>::from_str(topic_prefix)
                .and_then(|mut s| s.push_str(topic_suffix).map(|_| s))
                .unwrap_or_else(|_| panic!("failed to construct topic"))
        };


        let topic_commands = topic(MQTT_TOPIC_POSTFIX_COMMAND);
        let topic_wind_speed = topic(MQTT_TOPIC_POSTFIX_WIND_SPEED);
        #[allow(unused)]
        let topic_wind_angle = topic(MQTT_TOPIC_POSTFIX_WIND_DIRECTION);
    */
    let mut app_event = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();
    let mut app_data = APPLICATION_DATA_CHANNEL.subscriber().unwrap();
    let mut cmd_topic = String::new();
    let mut shadow_update_topic = String::new();
    let mut device_id = String::new();
    let mut boot_timestamp = datetime::get_datetime().unwrap();

    {
        info!("before lock()");
        let aws_config = super::super::AWSCONFIG.lock().unwrap();
        info!("after lock()");
        // need to remove tailing zeros otherwise CString will complain
        let topic_prefix = core::str::from_utf8(
            &(aws_config.topic_prefix[0..aws_config
                .topic_prefix
                .iter()
                .position(|&x| x == 0)
                .unwrap()]),
        )
        .unwrap();
        let topic_postfix = core::str::from_utf8(
            &(aws_config.cmd_topic_postfix[0..aws_config
                .cmd_topic_postfix
                .iter()
                .position(|&x| x == 0)
                .unwrap()]),
        )
        .unwrap();

        let device_id_str = core::str::from_utf8(
            &(aws_config.device_id[0..aws_config.device_id.iter().position(|&x| x == 0).unwrap()]),
        )
        .unwrap();
        device_id.push_str(device_id_str);

        // build topic string specific for device id
        cmd_topic.push_str(topic_prefix);
        cmd_topic.push('/');
        cmd_topic.push_str(device_id_str);
        cmd_topic.push_str(topic_postfix);
        info!("subscribing to {cmd_topic}");

        // need to remove tailing zeros otherwise CString will complain
        let shadow_update_postfix_str = core::str::from_utf8(
            &(aws_config.shadow_update_postfix[0..aws_config
                .shadow_update_postfix
                .iter()
                .position(|&x| x == 0)
                .unwrap()]),
        )
        .unwrap();

        let things_prefix_str = core::str::from_utf8(
            &(aws_config.things_prefix[0..aws_config
                .things_prefix
                .iter()
                .position(|&x| x == 0)
                .unwrap()]),
        )
        .unwrap();

        shadow_update_topic.push_str(things_prefix_str);
        shadow_update_topic.push('/');
        shadow_update_topic.push_str(device_id_str);
        shadow_update_topic.push_str(shadow_update_postfix_str);
        info!("posting to {shadow_update_topic}");
    }

    loop {
        let (conn_state, app_state_change, app_data) = match select3(
            MQTT_CONNECT_SIGNAL.wait(),
            app_event.next_message_pure(),
            app_data.next_message_pure(),
        )
        .await
        {
            Either3::First(conn_state) => {
                info!("send_task recv MQTT_CONNECT_SIGNAL");
                (Some(conn_state), None, None)
            }
            Either3::Second(app_state_change) => {
                info!("send_task recv app_state_change");
                (None, Some(app_state_change), None)
            }
            Either3::Third(app_data) => {
                info!("send_task recv app_state_change");
                (None, None, Some(app_data))
            }
        };

        if let Some(new_conn_state) = conn_state {
            if new_conn_state {
                info!("send_task MQTT is now connected, subscribing {}", cmd_topic);
                match mqtt.subscribe(cmd_topic.as_str(), QoS::AtLeastOnce).await {
                    Ok(_) => {
                        connected = true;
                    }
                    Err(err) => {
                        error!("Subscribe failed: {:?}", err);
                        connected = false;
                    }
                }
            } else {
                info!("send_task MQTT disconnected");
                connected = false;
            }
        }

        if let Some(ApplicationStateChange::OTAUpdateStarted) = app_state_change {
            info!("send_task OTA Update started shutting down mqtt send_task");
            drop(mqtt);
            break;
        }
        if let Some(ApplicationDataChange::ReportWindData) = app_data {
            let mut avg_speed = 0.0;
            let mut wind_gust = 0.0;

            if let Ok(mut wind_historian) = (*WIND_DATA_HISTORY).lock() {
                avg_speed = wind_historian.avg_speed();
                wind_gust = wind_historian.gust_speed();
                wind_historian.clear_wind_gust();
            };

            info!("send_task send wind speed = {avg_speed}, wind gust = {wind_gust}");

            if connected {
                if let Ok(now) = datetime::get_datetime() {
                    // check if we have a valid system time
                    if now.year() > 1970 {
                        let format = time::format_description::parse(
                            "[day].[month].[year] [hour]:[minute]:[second]",
                        )
                        .expect("Invalid format.");

                        if boot_timestamp.year() == 1970 {
                            boot_timestamp = now;
                        }

                        let time = now.format(&format).expect("Could not format time.");
                        let boot_time = boot_timestamp
                            .format(&format)
                            .expect("Could not format time.");
                        let avg_speed_string = format!("{avg_speed:.2}").trim().replace('.', ",");
                        let wind_gust_string = format!("{wind_gust:.2}").trim().replace('.', ",");
                        let epoch = (SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64)
                            .to_string();

                        let msg = AWSShadowUpdate {
                            windDirText: "NN",
                            deviceId: device_id.as_str(),
                            timeStamp: time.as_str(),
                            epochTime: epoch.as_str(),
                            bootTimeStamp: boot_time.as_str(),
                            windDir: "0,0",
                            windSpeed: avg_speed_string.as_str(),
                            windGust: wind_gust_string.as_str(),
                        };
                        let mut buffer: String = String::new();

                        info!("Posting update to {}", shadow_update_topic);
                        msg.format_aws_device_update_msg(&mut buffer);
                        // TODO: check error handling this panics
                        if let Ok(_msg_id) = error::check!(
                            mqtt.publish(
                                shadow_update_topic.as_str(),
                                QoS::AtLeastOnce,
                                false,
                                buffer.as_bytes()
                            )
                            .await
                        ) {
                            info!("send_task published to {}", shadow_update_topic.as_str());
                        } else {
                            connected = false;
                            error!(
                                "send_task failed to publish to {}",
                                shadow_update_topic.as_str()
                            );
                        }
                    } else {
                        info!("no vaild system time");
                    }
                }
            } else {
                info!(
                    "send_task client not connected, skipping publishment to {}",
                    shadow_update_topic.as_str()
                );
            }
        }
    }
}
