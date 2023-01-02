use crate::state::OtaUrl;
use core::str;
use embedded_svc::mqtt::client::asynch::{Event, Message};
use embedded_svc::mqtt::client::Details;
use log::*;
use serde::{Deserialize, Serialize};

pub const MQTT_TOPIC_POSTFIX_COMMAND: &str = "/command/#";
pub const MQTT_TOPIC_POSTFIX_COMMAND_OTA_UPDATE: &str = "/command/ota_update";
pub const MQTT_TOPIC_POSTFIX_COMMAND_SYSTEM_RESTART: &str = "/command/system_restart";
pub const MQTT_TOPIC_POSTFIX_WIND_SPEED: &str = "/wind/speed";
pub const MQTT_TOPIC_POSTFIX_WIND_DIRECTION: &str = "/wind/direction";

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum MqttCommand {
    ExecOTAUpdate(OtaUrl),
    SystemRestart,
}

pub struct MessageParser {
    #[allow(clippy::type_complexity)]
    command_parser: Option<fn(&[u8]) -> Option<MqttCommand>>,
    payload_buf: [u8; 128],
}

impl MessageParser {
    pub fn new() -> Self {
        MessageParser {
            command_parser: None,
            payload_buf: [0; 128],
        }
    }

    pub fn convert<M, E>(
        &mut self,
        event: &Result<Event<M>, E>,
    ) -> Result<Event<Option<MqttCommand>>, E>
    where
        M: Message,
        E: Clone,
    {
        event
            .as_ref()
            .map(|event| event.transform_received(|message| self.process(message)))
            .map_err(|e| e.clone())
    }

    fn process<M>(&mut self, message: &M) -> Option<MqttCommand>
    where
        M: Message,
    {
        info!("Message = {:?}", message.details());
        match message.details() {
            Details::Complete => Self::parse_command(message.topic().unwrap())
                .and_then(|parser| parser(message.data())),
            Details::InitialChunk(initial_chunk_data) => {
                if initial_chunk_data.total_data_size > self.payload_buf.len() {
                    self.command_parser = None;
                } else {
                    self.command_parser = Self::parse_command(message.topic().unwrap());

                    self.payload_buf[..message.data().len()]
                        .copy_from_slice(message.data().as_ref());
                }

                None
            }
            Details::SubsequentChunk(subsequent_chunk_data) => {
                if let Some(command_parser) = self.command_parser.as_ref() {
                    self.payload_buf
                        [subsequent_chunk_data.current_data_offset..message.data().len()]
                        .copy_from_slice(message.data().as_ref());

                    if subsequent_chunk_data.total_data_size
                        == subsequent_chunk_data.current_data_offset + message.data().len()
                    {
                        command_parser(&self.payload_buf[0..subsequent_chunk_data.total_data_size])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    #[allow(clippy::type_complexity)]
    fn parse_command(topic: &str) -> Option<fn(&[u8]) -> Option<MqttCommand>> {
        info!("parse_command: {}", topic);
        if topic.ends_with(MQTT_TOPIC_POSTFIX_COMMAND_OTA_UPDATE) {
            Some(Self::parse_ota_update_command)
        } else if topic.ends_with(MQTT_TOPIC_POSTFIX_COMMAND_SYSTEM_RESTART) {
            Some(Self::parse_system_restart_command)
        } else {
            None
        }
    }

    fn parse_ota_update_command(data: &[u8]) -> Option<MqttCommand> {
        Self::parse::<OtaUrl>(data).map(MqttCommand::ExecOTAUpdate)
    }

    fn parse_system_restart_command(data: &[u8]) -> Option<MqttCommand> {
        Self::parse_empty(data).map(|_| MqttCommand::SystemRestart)
    }

    fn parse<T>(data: &[u8]) -> Option<T>
    where
        T: str::FromStr,
    {
        str::from_utf8(data)
            .ok()
            .and_then(|s| str::parse::<T>(s).ok())
    }

    fn parse_empty(data: &[u8]) -> Option<()> {
        if data.is_empty() {
            Some(())
        } else {
            None
        }
    }
}
