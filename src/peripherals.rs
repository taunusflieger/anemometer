use esp_idf_hal::gpio::*;
use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::CHANNEL0;

#[cfg(any(feature = "sdcard", feature = "tft"))]
use esp_idf_hal::spi::*;

#[cfg(feature = "gps")]
use esp_idf_hal::uart::*;

#[cfg(feature = "tft")]
pub struct SystemPeripherals<VDD, NEOPIXELPIN, CHANNEL> {
    pub neopixel: NeoPixelPeripherals<NEOPIXELPIN, CHANNEL>,

    pub display: DisplaySpiPeripherals<VDD>,
    pub display_backlight: AnyOutputPin,

    #[cfg(feature = "sdcard")]
    pub sdcard: MicroSDCardPeripherals,

    #[cfg(feature = "gps")]
    pub gps: GpsPeripherals,

    #[cfg(any(feature = "sdcard", feature = "tft"))]
    pub spi_bus: SpiBusPeripherals,

    pub pulse_counter: AnemometerPulseCounterPeripherals,
    pub modem: Modem,
}

#[cfg(all(not(feature = "tft"), feature = "neopixel"))]
pub struct SystemPeripherals<NEOPIXELPIN, CHANNEL> {
    pub neopixel: NeoPixelPeripherals<NEOPIXELPIN, CHANNEL>,

    #[cfg(feature = "sdcard")]
    pub sdcard: MicroSDCardPeripherals,

    #[cfg(feature = "gps")]
    pub gps: GpsPeripherals,

    #[cfg(feature = "sdcard")]
    pub spi_bus: SpiBusPeripherals,

    pub pulse_counter: AnemometerPulseCounterPeripherals,
    pub modem: Modem,
}

#[cfg(feature = "production")]
pub struct SystemPeripherals {
    pub pulse_counter: AnemometerPulseCounterPeripherals,
    pub modem: Modem,
}

#[cfg(feature = "production")]
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

#[cfg(feature = "tft")]
impl SystemPeripherals<Gpio21, Gpio33, CHANNEL0> {
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

            #[cfg(feature = "tft")]
            display: DisplaySpiPeripherals {
                control: DisplayControlPeripherals {
                    dc: peripherals.pins.gpio39.into(),
                    rst: peripherals.pins.gpio40.into(),
                },
                cs: peripherals.pins.gpio7.into(),
                vdd: peripherals.pins.gpio21,
            },

            #[cfg(feature = "tft")]
            display_backlight: peripherals.pins.gpio45.into(),

            #[cfg(feature = "sdcard")]
            sdcard: MicroSDCardPeripherals {
                cs: peripherals.pins.gpio10.into(), // TODO: check
            },

            #[cfg(feature = "gps")]
            gps: GpsPeripherals {
                tx: peripherals.pins.gpio1.into(),
                rx: peripherals.pins.gpio2.into(),
                uart1: peripherals.uart1,
            },
            pulse_counter: AnemometerPulseCounterPeripherals {
                pulse: peripherals.pins.gpio5.into(),
            },

            #[cfg(any(feature = "sdcard", feature = "tft"))]
            spi_bus: SpiBusPeripherals {
                driver: std::rc::Rc::new(
                    SpiDriver::new(
                        peripherals.spi2,
                        peripherals.pins.gpio36,
                        peripherals.pins.gpio35,
                        Some(peripherals.pins.gpio37),
                        Dma::Disabled,
                    )
                    .unwrap(),
                ),
            },
            modem: peripherals.modem,
        }
    }
}

#[cfg(all(not(feature = "tft"), feature = "neopixel"))]
impl SystemPeripherals<Gpio33, CHANNEL0> {
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

            #[cfg(feature = "sdcard")]
            sdcard: MicroSDCardPeripherals {
                cs: peripherals.pins.gpio10.into(), // TODO: check
            },

            #[cfg(feature = "gps")]
            gps: GpsPeripherals {
                tx: peripherals.pins.gpio1.into(),
                rx: peripherals.pins.gpio2.into(),
                uart1: peripherals.uart1,
            },
            pulse_counter: AnemometerPulseCounterPeripherals {
                pulse: peripherals.pins.gpio5.into(),
            },

            #[cfg(feature = "sdcard")]
            spi_bus: SpiBusPeripherals {
                driver: std::rc::Rc::new(
                    SpiDriver::new(
                        peripherals.spi2,
                        peripherals.pins.gpio36,
                        peripherals.pins.gpio35,
                        Some(peripherals.pins.gpio37),
                        Dma::Disabled,
                    )
                    .unwrap(),
                ),
            },
            modem: peripherals.modem,
        }
    }
}

#[cfg(feature = "neopixel")]
pub struct NeoPixelPeripherals<NEOPIXELPIN, CHANNEL> {
    pub dc: AnyOutputPin,
    pub pin: NEOPIXELPIN,
    pub channel: CHANNEL,
}

#[cfg(feature = "tft")]
pub struct DisplayControlPeripherals {
    pub dc: AnyOutputPin,
    pub rst: AnyOutputPin,
}
#[cfg(feature = "tft")]
pub struct DisplaySpiPeripherals<VDD> {
    pub control: DisplayControlPeripherals,
    pub cs: AnyOutputPin,
    pub vdd: VDD,
}

#[cfg(feature = "gps")]
pub struct GpsPeripherals {
    pub tx: AnyOutputPin,
    pub rx: AnyInputPin,
    pub uart1: UART1,
}

#[cfg(any(feature = "sdcard", feature = "tft"))]
pub struct SpiBusPeripherals {
    pub driver: std::rc::Rc<SpiDriver<'static>>,
}

#[cfg(feature = "sdcard")]
pub struct MicroSDCardPeripherals {
    pub cs: AnyOutputPin,
}

pub struct AnemometerPulseCounterPeripherals {
    pub pulse: AnyIOPin,
}
