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
pub mod ws2812 {
    use crate::peripherals::NeoPixelPeripherals;
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
        vdd: PinDriver<'d, AnyOutputPin, Output>,
        high: (Pulse, Pulse),
        low: (Pulse, Pulse),
    }

    fn ns(nanos: u64) -> Duration {
        Duration::from_nanos(nanos)
    }

    impl<'d> NeoPixel<'d> {
        pub fn new<C>(
            neopixel_peripherals: NeoPixelPeripherals<
                impl Peripheral<P = impl OutputPin + 'static> + 'static,
                impl Peripheral<P = C> + 'd,
            >,
        ) -> Result<NeoPixel<'d>, EspError>
        where
            C: RmtChannel,
        {
            let mut vdd: PinDriver<AnyOutputPin, Output> =
                PinDriver::output(neopixel_peripherals.dc).unwrap();
            vdd.set_high()?;

            let config = TransmitConfig::new().clock_divider(1);
            let tx = match TxRmtDriver::new(
                neopixel_peripherals.channel,
                neopixel_peripherals.pin,
                &config,
            ) {
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
                vdd,
                high: (t1h, t1l),
                low: (t0h, t0l),
            })
        }

        pub fn power_on(&mut self, set: bool) {
            if set {
                self.vdd.set_high().expect("neopixel led power on failed");
            } else {
                self.vdd.set_low().expect("neopixel led power off failed");
            }
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
