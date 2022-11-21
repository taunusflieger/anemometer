use esp_idf_svc::errors::EspIOError;
use esp_idf_sys::EspError;

#[derive(Debug)]
pub enum InitError {
    EspError(EspError),
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
