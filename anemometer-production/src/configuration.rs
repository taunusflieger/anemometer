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
use esp_idf_svc::nvs::*;
use esp_idf_sys::*;

#[derive(Debug)]
pub struct AwsIoTSettings {
    pub ca_cert: [u8; 2048],
    pub cert: [u8; 2048],
    pub priv_key: [u8; 2048],
    pub shadow_update: [u8; 128],
    pub shadow_delta: [u8; 128],
    pub shadow_documents: [u8; 128],
    pub host: [u8; 128],
    pub port: u16,
    pub device_id: [u8; 32],
}

impl AwsIoTSettings {
    pub fn new(partition: &str) -> Result<Self, EspError> {
        let mut settings = AwsIoTSettings {
            ca_cert: [0; 2048],
            cert: [0; 2048],
            priv_key: [0; 2048],
            shadow_update: [0; 128],
            shadow_delta: [0; 128],
            shadow_documents: [0; 128],
            host: [0; 128],
            port: 0,
            device_id: [0; 32],
        };

        let part = EspCustomNvsPartition::take(partition)?;
        let nvs = EspCustomNvs::new(part.clone(), "certificates", false)?;
        nvs.get_str("ca_cert", &mut settings.ca_cert)?;
        nvs.get_str("cert", &mut settings.cert)?;
        nvs.get_str("priv_key", &mut settings.priv_key)?;

        let nvs = EspCustomNvs::new(part.clone(), "aws_settings", false)?;
        nvs.get_str("shadow_update", &mut settings.shadow_update)?;
        nvs.get_str("shadow_delta", &mut settings.shadow_delta)?;
        nvs.get_str("shadow_doc", &mut settings.shadow_documents)?;
        nvs.get_str("host", &mut settings.host)?;

        let mut port_buf: [u8; 6] = [0; 6];
        nvs.get_str("port", &mut port_buf)?;
        let s = std::str::from_utf8(&port_buf).unwrap();
        settings.port = (s[..s.len() - 2]).parse::<u16>().unwrap();

        let nvs = EspCustomNvs::new(part, "device_data", false)?;
        nvs.get_str("device_id", &mut settings.device_id)?;

        Ok(settings)
    }
}
