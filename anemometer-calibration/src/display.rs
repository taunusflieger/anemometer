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
use crate::errors::*;
use crate::peripherals::DisplaySpiPeripherals;
use core::fmt::Debug;
use core::mem;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::pixelcolor::Rgb565;
use esp_idf_hal::delay;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::*;
use gfx_xtra::draw_target::{Flushable, OwnedDrawTargetExt};
use mipidsi::{Builder, Orientation};

pub fn display(
    display_peripherals: DisplaySpiPeripherals<
        impl Peripheral<P = impl OutputPin + 'static> + 'static,
    >,
    driver: std::rc::Rc<SpiDriver<'static>>,
) -> Result<impl Flushable<Color = Rgb565, Error = impl Debug + 'static> + 'static, InitError> {
    // power ST7789
    let mut vdd = PinDriver::output(display_peripherals.vdd)?;
    vdd.set_high()?;
    mem::forget(vdd);

    let spi_display = SpiDeviceDriver::new(
        driver,
        Some(display_peripherals.cs),
        &SpiConfig::new().baudrate(10.MHz().into()),
    )
    .unwrap();

    let dc = PinDriver::output(display_peripherals.control.dc).unwrap();
    let di = SPIInterfaceNoCS::new(spi_display, dc);
    let rst = PinDriver::output(display_peripherals.control.rst).unwrap();

    // create driver
    let display = Builder::st7789_pico1(di)
        .with_display_size(135, 240)
        // set default orientation
        .with_orientation(Orientation::Landscape(true))
        // initialize
        .init(&mut delay::Ets, Some(rst))
        .unwrap();

    let display = display.owned_noop_flushing();
    Ok(display)
}
