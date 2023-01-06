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
use crate::data_processing::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::PubSubChannel;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

lazy_static! {
    pub static ref WIND_DATA_HISTORY: Arc<Mutex<WindDataHistory>> =
        Arc::new(Mutex::new(WindDataHistory::default()));
}

use serde::{Deserialize, Serialize};

pub type OtaUrl = heapless::String<128>;

pub static NETWORK_EVENT_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    NetworkStateChange,
    4,
    4,
    4,
> = PubSubChannel::new();

#[allow(dead_code)]
pub static APPLICATION_EVENT_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    ApplicationStateChange,
    5,
    5,
    5,
> = PubSubChannel::new();

#[allow(dead_code)]
pub static APPLICATION_DATA_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    ApplicationDataChange,
    5,
    5,
    5,
> = PubSubChannel::new();

#[derive(Copy, Clone, Debug)]
pub enum NetworkStateChange {
    WifiDisconnected,
    IpAddressAssigned { ip: embedded_svc::ipv4::Ipv4Addr },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct WindData {
    pub speed: u16,
    pub angle: u16,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ApplicationStateChange {
    OTAUpdateRequest(OtaUrl),
    OTAUpdateStarted,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ApplicationDataChange {
    ReportWindData,
}
