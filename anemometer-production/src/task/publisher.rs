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
use crate::global_settings;
use crate::state::*;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use log::*;

pub async fn wind_speed_task() {
    let publisher = APPLICATION_DATA_CHANNEL.publisher().unwrap();
    let mut app_event = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();
    info!("Publisher Task Started");
    loop {
        let (timer_fired, app_state_change) = match select(
            Timer::after(Duration::from_secs(
                global_settings::DATA_REPORTING_INTERVAL,
            )),
            app_event.next_message_pure(),
        )
        .await
        {
            Either::First(_) => (Some(true), None),
            Either::Second(app_state_change) => (None, Some(app_state_change)),
        };
        if let Some(ApplicationStateChange::OTAUpdateStarted) = app_state_change {
            info!(
                "wind_speed_task OTA Update started shutting down wind_speed_demo_publisher task"
            );
            break;
        }

        if let Some(send_needed) = timer_fired {
            if send_needed {
                publisher
                    .publish(ApplicationDataChange::ReportWindData)
                    .await;
            }
        }
    }
}
