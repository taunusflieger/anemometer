pub mod ws2812 {
    use esp_idf_hal::gpio::*;
    use esp_idf_hal::peripheral::Peripheral;
    use esp_idf_hal::rmt::config::TransmitConfig;
    use esp_idf_hal::rmt::RmtChannel;
    use esp_idf_hal::rmt::{FixedLengthSignal, PinState, Pulse, TxRmtDriver};
    use esp_idf_sys::EspError;
    use smart_leds::RGB8;
    use std::time::Duration;

    pub struct NeoPixel<'d> {
        tx: TxRmtDriver<'d>,
        high: (Pulse, Pulse),
        low: (Pulse, Pulse),
    }

    fn ns(nanos: u64) -> Duration {
        Duration::from_nanos(nanos)
    }

    impl<'d> NeoPixel<'d> {
        pub fn new<C>(
            channel: impl Peripheral<P = C> + 'd,
            pin: impl Peripheral<P = impl OutputPin> + 'd,
        ) -> Result<NeoPixel<'d>, EspError>
        where
            C: RmtChannel,
        {
            let config = TransmitConfig::new().clock_divider(1);
            let tx = match TxRmtDriver::new(channel, pin, &config) {
                Ok(r) => r,
                Err(e) => panic!("Problem ccreate TxRmtDriver: {:?}", e),
            };

            let ticks_hz = tx.counter_clock()?;
            let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350))?;
            let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800))?;
            let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700))?;
            let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600))?;
            Ok(NeoPixel {
                tx,
                high: (t1h, t1l),
                low: (t0h, t0l),
            })
        }

        pub fn set_blocking_rgb(&mut self, r: u8, g: u8, b: u8) -> Result<(), EspError> {
            let rgb = (b as u32) << 16 | (r as u32) << 8 | g as u32;
            self.set_blocking(rgb)
        }

        pub fn set_blocking(&mut self, rgb: u32) -> Result<(), EspError> {
            let mut signal = FixedLengthSignal::<24>::new();
            for i in 0..24 {
                let bit = 2_u32.pow(i) & rgb != 0;
                let bit = if bit { self.high } else { self.low };
                signal.set(i as usize, &bit)?;
            }
            self.tx
                .start_blocking(&signal)
                .expect("Rmt sending sequence failed");
            Ok(())
        }

        pub fn write(&mut self, color: RGB8) -> Result<(), EspError> {
            self.set_blocking_rgb(color.r, color.g, color.b)
        }
    }
}
