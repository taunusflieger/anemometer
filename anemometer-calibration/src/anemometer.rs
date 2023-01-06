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
