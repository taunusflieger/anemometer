use esp_idf_svc::sntp::*;
use esp_idf_sys::EspError;
use log::*;
use std::{convert::TryFrom, time::SystemTime};
use time::*;

pub fn initialize() -> core::result::Result<esp_idf_svc::sntp::EspSntp, EspError> {
    let sntp = esp_idf_svc::sntp::EspSntp::new_default()?;

    // set time zone for Berlin/Germany
    // taken from here https://sites.google.com/a/usapiens.com/opnode/time-zones
    let german_tz = std::ffi::CString::new("CET-1CEST-2,M3.5.0/02:00:00,M10.5.0/03:00:00").unwrap();
    let tz_var = std::ffi::CString::new("TZ").unwrap();
    unsafe {
        esp_idf_sys::setenv(tz_var.as_ptr(), german_tz.as_ptr(), 1);
        esp_idf_sys::tzset();
    }

    Ok(sntp)
}

pub fn get_datetime() -> Result<PrimitiveDateTime> {
    let unixtime = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let tm = unsafe { *esp_idf_sys::localtime(&(unixtime.as_secs() as i64)) };
    let month = Month::try_from(1u8 + tm.tm_mon as u8)?;
    let date = Date::from_calendar_date(1900 + tm.tm_year, month, tm.tm_mday as _)?;
    let time = Time::from_hms(tm.tm_hour as _, tm.tm_min as _, tm.tm_sec as _)?;

    Ok(PrimitiveDateTime::new(date, time))
}
