pub mod anemometer {
    static ANEMOMETER_PULSCOUNT: AtomicU32 = AtomicU32::new(0);
    const MEASUREMENT_INTERVAL: u64 = 5;

    use crate::errors::*;
    use esp_idf_hal::gpio::*;
    use esp_idf_hal::peripheral::Peripheral;
    use esp_idf_svc::timer::*;
    use esp_idf_sys::*;
    use fixed::types::U20F12;
    use std::sync::{atomic::*, Arc};
    use std::time::Duration;

    pub struct AnemometerData<P>
    where
        P: Pin,
    {
        pub rps: Arc<AtomicU32>,
        pub angle: f32,
        _pin: PinDriver<'static, P, Input>,
    }

    impl<'d, P: InputPin> AnemometerData<P> {
        pub fn new(pin: impl Peripheral<P = P> + 'static) -> Result<AnemometerData<P>, InitError> {
            Ok(AnemometerData {
                rps: Arc::new(AtomicU32::new(0)),
                angle: 0.0,
                _pin: subscribe_pin(pin, count_pulse)?,
            })
        }

        pub fn set_measurement_timer(&mut self) -> Result<EspTimer, EspError> {
            let rps_store = Arc::clone(&self.rps);
            let periodic_timer = EspTimerService::new()?.timer(move || {
                let cnt = ANEMOMETER_PULSCOUNT.fetch_and(0, Ordering::Relaxed);
                let rps = U20F12::from_num(cnt) / 2 / (MEASUREMENT_INTERVAL as u32); // two pules per 360 degree rotation
                rps_store.store(rps.to_bits(), Ordering::Relaxed);
            })?;

            periodic_timer.every(Duration::from_secs(MEASUREMENT_INTERVAL))?;

            Ok(periodic_timer)
        }

        pub fn get_current_rps(&self) -> f32 {
            U20F12::from_bits(self.rps.load(Ordering::Relaxed)).to_num()
        }
    }

    fn count_pulse() {
        ANEMOMETER_PULSCOUNT.fetch_add(1, Ordering::Relaxed);
    }

    fn subscribe_pin<'d, P: InputPin>(
        pin: impl Peripheral<P = P> + 'd,
        notify: impl Fn() + 'static,
    ) -> Result<PinDriver<'d, P, Input>, InitError> {
        let mut pin = PinDriver::input(pin)?;

        pin.set_interrupt_type(InterruptType::NegEdge)?;

        unsafe {
            pin.subscribe(notify)?;
        }
        Ok(pin)
    }
}
