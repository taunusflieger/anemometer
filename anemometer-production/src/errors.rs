use edge_executor::SpawnError;

use core::fmt;
use esp_idf_svc::errors::EspIOError;
use esp_idf_sys::EspError;

#[derive(Debug)]
pub enum OtaError {
    FwImageNotFound,
    FwSameAsInvalidFw,
    VersionAlreadyFlashed,
    FlashFailed,
    ImageLoadIncomplete,
    HttpError,
    OtaApiError,
}

impl fmt::Display for OtaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FwImageNotFound => write!(
                f,
                "Firmware not found on server or unexcpected server response error"
            ),
            Self::FwSameAsInvalidFw => write!(f, "New firmware same as invalid marked firmware"),
            Self::VersionAlreadyFlashed => write!(f, "Firmware with same version already flashed"),
            Self::HttpError => write!(f, "Calling Http client API error"),
            Self::OtaApiError => write!(f, "Calling OTA API error"),
            Self::FlashFailed => write!(f, "Failed to write FW data to flash"),
            Self::ImageLoadIncomplete => write!(f, "Failed to download complete FW image"),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum InitError {
    EspError(EspError),
    SpawnError(SpawnError),
    OtaError(OtaError),
}

impl From<EspError> for InitError {
    fn from(e: EspError) -> Self {
        Self::EspError(e)
    }
}

impl From<EspIOError> for InitError {
    fn from(e: EspIOError) -> Self {
        Self::EspError(e.0)
    }
}

impl From<SpawnError> for InitError {
    fn from(e: SpawnError) -> Self {
        Self::SpawnError(e)
    }
}

impl From<OtaError> for InitError {
    fn from(e: OtaError) -> Self {
        Self::OtaError(e)
    }
}
