/*
 * ESP32 Anemometer
 *
 * MIT license
 *
 * Copyright (c) 2021-2023 Michael Zill
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 * Apache license, Version 2.0
 *
 * Copyright (c) 2021-2023 Michael Zill
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use esp_idf_hal::gpio::*;
use esp_idf_hal::modem::Modem;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::CHANNEL0;
use esp_idf_hal::spi::*;
use esp_idf_hal::uart::*;

pub struct SystemPeripherals<VDD, NEOPIXELPIN, CHANNEL> {
    pub neopixel: NeoPixelPeripherals<NEOPIXELPIN, CHANNEL>,

    pub display: DisplaySpiPeripherals<VDD>,
    pub display_backlight: AnyOutputPin,

    pub sdcard: MicroSDCardPeripherals,

    pub gps: GpsPeripherals,

    pub spi_bus: SpiBusPeripherals,

    pub pulse_counter: AnemometerPulseCounterPeripherals,
    pub modem: Modem,
}

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
                pin: peripherals.pins.gpio33,
                channel: peripherals.rmt.channel0,
            },

            display: DisplaySpiPeripherals {
                control: DisplayControlPeripherals {
                    dc: peripherals.pins.gpio39.into(),
                    rst: peripherals.pins.gpio40.into(),
                },
                cs: peripherals.pins.gpio7.into(),
                vdd: peripherals.pins.gpio21,
            },

            display_backlight: peripherals.pins.gpio45.into(),

            sdcard: MicroSDCardPeripherals {
                cs: peripherals.pins.gpio10.into(), // TODO: check
            },

            gps: GpsPeripherals {
                tx: peripherals.pins.gpio1.into(),
                rx: peripherals.pins.gpio2.into(),
                uart1: peripherals.uart1,
            },
            pulse_counter: AnemometerPulseCounterPeripherals {
                pulse: peripherals.pins.gpio5.into(),
            },

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
pub struct NeoPixelPeripherals<NEOPIXELPIN, CHANNEL> {
    pub dc: AnyOutputPin,
    pub pin: NEOPIXELPIN,
    pub channel: CHANNEL,
}

pub struct DisplayControlPeripherals {
    pub dc: AnyOutputPin,
    pub rst: AnyOutputPin,
}

pub struct DisplaySpiPeripherals<VDD> {
    pub control: DisplayControlPeripherals,
    pub cs: AnyOutputPin,
    pub vdd: VDD,
}

pub struct GpsPeripherals {
    pub tx: AnyOutputPin,
    pub rx: AnyInputPin,
    pub uart1: UART1,
}

pub struct SpiBusPeripherals {
    pub driver: std::rc::Rc<SpiDriver<'static>>,
}

pub struct MicroSDCardPeripherals {
    pub cs: AnyOutputPin,
}

pub struct AnemometerPulseCounterPeripherals {
    pub pulse: AnyIOPin,
}
