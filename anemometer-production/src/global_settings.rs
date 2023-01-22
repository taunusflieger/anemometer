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
// Global setting for data aquisition and reporting [sec]
pub const DATA_REPORTING_INTERVAL: u64 = 120;
// Interval for taking measurments from the anemometer [ms]
pub const MEASUREMENT_INTERVAL: u64 = 500;
// light sleep mode max cpu frequency
pub const MAX_CPU_FREQ: i32 = 160;
// light sleep mode min cpu frequency
pub const MIN_CPU_FREQ: i32 = 40;
// light sleep mode enabled flag
pub const LIGHT_SLEEP_MODE_ENABLED: bool = true;
// All user task will run on core 1
// core 0 will be used by wifi and network stack
pub const TASK_HIGH_PRIORITY: u8 = 30;
pub const TASK_MID_PRIORITY: u8 = 27;
pub const TASK_LOW_PRIORITY: u8 = 25;
pub const MQTT_MAX_TOPIC_LEN: usize = 64;
