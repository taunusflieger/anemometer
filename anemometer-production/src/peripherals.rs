use esp_idf_hal::gpio::*;
use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripherals::Peripherals;

pub struct SystemPeripherals {
    pub pulse_counter: AnemometerPulseCounterPeripherals,
    pub modem: Modem,
}

impl SystemPeripherals {
    pub fn take() -> Self {
        let peripherals = Peripherals::take().unwrap();

        SystemPeripherals {
            pulse_counter: AnemometerPulseCounterPeripherals {
                pulse: peripherals.pins.gpio5.into(),
            },

            modem: peripherals.modem,
        }
    }
}

pub struct AnemometerPulseCounterPeripherals {
    pub pulse: AnyIOPin,
}
