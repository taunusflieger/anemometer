use crate::error;
use crate::mqtt_msg::{
    MqttCommand, MQTT_TOPIC_POSTFIX_COMMAND, MQTT_TOPIC_POSTFIX_WIND_DIRECTION,
    MQTT_TOPIC_POSTFIX_WIND_SPEED,
};
use crate::state::*;
use core::str::{self, FromStr};
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embedded_svc::mqtt::client::asynch::{Client, Connection, Event, Publish, QoS};
use log::*;

static MQTT_CONNECT_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

pub async fn receive_task(mut connection: impl Connection<Message = Option<MqttCommand>>) {
    let mut app_event = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();

    loop {
        let (message, app_state_change) =
            match select(connection.next(), app_event.next_message_pure()).await {
                Either::First(message) => {
                    info!("send_task recv MQTT_CONNECT_SIGNAL");
                    (message, None)
                }
                Either::Second(app_state_change) => {
                    info!("send_task recv app_state_change");
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
pub async fn send_task<const L: usize>(topic_prefix: &str, mut mqtt: impl Client + Publish) {
    let mut connected = false;

    let topic = |topic_suffix| {
        heapless::String::<L>::from_str(topic_prefix)
            .and_then(|mut s| s.push_str(topic_suffix).map(|_| s))
            .unwrap_or_else(|_| panic!("failed to construct topic"))
    };

    let topic_commands = topic(MQTT_TOPIC_POSTFIX_COMMAND);
    let topic_wind_speed = topic(MQTT_TOPIC_POSTFIX_WIND_SPEED);
    #[allow(unused)]
    let topic_wind_angle = topic(MQTT_TOPIC_POSTFIX_WIND_DIRECTION);

    let mut app_event = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();
    let mut app_data = APPLICATION_DATA_CHANNEL.subscriber().unwrap();

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
                info!("send_task MQTT is now connected, subscribing");

                mqtt.subscribe(topic_commands.as_str(), QoS::AtLeastOnce)
                    .await
                    .unwrap();

                connected = true;
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
        if let Some(ApplicationDataChange::NewWindData(wind_data)) = app_data {
            info!("send_task send new wind data {}", wind_data.speed);

            if connected {
                if let Ok(_msg_id) = error::check!(
                    mqtt.publish(
                        &topic_wind_speed,
                        QoS::AtLeastOnce,
                        false,
                        format!("{}", wind_data.speed).as_str().as_bytes()
                    )
                    .await
                ) {
                    info!("send_task published to {}", topic_wind_speed);
                }
            } else {
                info!(
                    "send_task client not connected, skipping publishment to {}",
                    topic_wind_speed
                );
            }
        }
    }
}
