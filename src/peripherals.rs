use esp_idf_hal::gpio::*;
use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::CHANNEL0;
pub struct SystemPeripherals {
    pub neopixel: NeoPixelPeripherals,
    pub modem: Modem,
}

//#[cfg(any(esp32s2, esp32s3))]
impl SystemPeripherals {
    pub fn take() -> Self {
        let peripherals = Peripherals::take().unwrap();

        SystemPeripherals {
            // Required for UM TinyS3 board. The WS2812 VDD pin is connected
            // to PIN 17, so it needs to be powered through the PIN
            // Required for Adafruit Feather ESP32-S3. The WS2812 VDD pin is connected
            // to PIN 21, so it needs to be powered through the PIN
            neopixel: NeoPixelPeripherals {
                dc: peripherals.pins.gpio21.into(),
                pin: peripherals.pins.gpio33.into(),
                channel: peripherals.rmt.channel0,
            },
            modem: peripherals.modem,
        }
    }
}

pub struct NeoPixelPeripherals {
    pub dc: AnyOutputPin,
    pub pin: AnyOutputPin,
    pub channel: CHANNEL0,
}
