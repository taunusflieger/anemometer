use esp_idf_hal::gpio::*;
use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::CHANNEL0;
use esp_idf_hal::spi::*;

pub struct SystemPeripherals<SPI, VDD, NEOPIXELPIN, CHANNEL> {
    pub neopixel: NeoPixelPeripherals<NEOPIXELPIN, CHANNEL>,
    pub display: DisplaySpiPeripherals<SPI, VDD>,
    pub modem: Modem,
    // pub display_rst: AnyOutputPin,
}

//#[cfg(any(esp32s2, esp32s3))]
impl SystemPeripherals<SPI2, Gpio21, Gpio33, CHANNEL0> {
    pub fn take() -> Self {
        let peripherals = Peripherals::take().unwrap();

        SystemPeripherals {
            // Required for UM TinyS3 board. The WS2812 VDD pin is connected
            // to PIN 17, so it needs to be powered through the PIN
            // Required for Adafruit Feather ESP32-S3. The WS2812 VDD pin is connected
            // to PIN 21, so it needs to be powered through the PIN
            // For the Adafruit Feather ESP32-S3 TFT VDD is connected to GPIO 34
            neopixel: NeoPixelPeripherals {
                dc: peripherals.pins.gpio34.into(),
                pin: peripherals.pins.gpio33.into(),
                channel: peripherals.rmt.channel0,
            },
            display: DisplaySpiPeripherals {
                control: DisplayControlPeripherals {
                    backlight: Some(peripherals.pins.gpio45.into()),
                    dc: peripherals.pins.gpio39.into(),
                    rst: peripherals.pins.gpio40.into(),
                },
                spi: peripherals.spi2,
                sclk: peripherals.pins.gpio36.into(),
                sdo: peripherals.pins.gpio35.into(),
                cs: peripherals.pins.gpio7.into(),
                vdd: peripherals.pins.gpio21,
            },
            modem: peripherals.modem,
            //display_rst: peripherals.pins.gpio40.into(),
        }
    }
}

pub struct NeoPixelPeripherals<NEOPIXELPIN, CHANNEL> {
    pub dc: AnyOutputPin,
    pub pin: NEOPIXELPIN,
    pub channel: CHANNEL,
}

pub struct DisplayControlPeripherals {
    pub backlight: Option<AnyOutputPin>,
    pub dc: AnyOutputPin,
    pub rst: AnyOutputPin,
}
pub struct DisplaySpiPeripherals<SPI, VDD> {
    pub control: DisplayControlPeripherals,
    pub spi: SPI,
    pub sclk: AnyOutputPin,
    pub sdo: AnyOutputPin,
    pub cs: AnyOutputPin,
    pub vdd: VDD,
}
