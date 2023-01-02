use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::PubSubChannel;
use serde::{Deserialize, Serialize};

pub type OtaUrl = heapless::String<128>;

pub static NETWORK_EVENT_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    NetworkStateChange,
    4,
    4,
    4,
> = PubSubChannel::new();

#[allow(dead_code)]
pub static APPLICATION_EVENT_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    ApplicationStateChange,
    5,
    5,
    5,
> = PubSubChannel::new();

#[allow(dead_code)]
pub static APPLICATION_DATA_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    ApplicationDataChange,
    5,
    5,
    5,
> = PubSubChannel::new();

#[derive(Copy, Clone, Debug)]
pub enum NetworkStateChange {
    WifiDisconnected,
    IpAddressAssigned { ip: embedded_svc::ipv4::Ipv4Addr },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct WindData {
    pub speed: u16,
    pub angle: u16,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ApplicationStateChange {
    OTAUpdateRequest(OtaUrl),
    OTAUpdateStarted,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ApplicationDataChange {
    NewWindData(WindData),
}
