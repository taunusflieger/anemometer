static ANEMOMETER_PULSCOUNT: AtomicU32 = AtomicU32::new(0);
const MEASUREMENT_INTERVAL: u64 = 5;

use crate::errors::*;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_svc::timer::*;
use esp_idf_sys::*;
use std::sync::{atomic::*, Mutex};
use std::time::Duration;

pub static GLOBAL_ANEMOMETER_DATA: Mutex<GlobalAnemometerData> = Mutex::new(GlobalAnemometerData {
    rps: 0.0,
    angle: 0.0,
});

pub struct GlobalAnemometerData {
    pub rps: f32,
    pub angle: f32,
}

pub struct AnemometerDriver<P>
where
    P: Pin,
{
    _pin: PinDriver<'static, P, Input>,
}

impl<P: InputPin + OutputPin> AnemometerDriver<P> {
    pub fn new(pin: impl Peripheral<P = P> + 'static) -> Result<AnemometerDriver<P>, InitError> {
        Ok(AnemometerDriver {
            _pin: subscribe_pin(pin, count_pulse)?,
        })
    }

    pub fn set_measurement_timer(&mut self) -> Result<EspTimer, EspError> {
        let periodic_timer = EspTimerService::new()?.timer(move || {
            let cnt = ANEMOMETER_PULSCOUNT.fetch_and(0, Ordering::Relaxed);
            let mut anemometer_data = GLOBAL_ANEMOMETER_DATA.lock().unwrap();
            anemometer_data.rps = cnt as f32 / 2.0 / (MEASUREMENT_INTERVAL as u32) as f32;
        })?;

        periodic_timer.every(Duration::from_secs(MEASUREMENT_INTERVAL))?;

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
