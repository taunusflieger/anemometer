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

#[derive(Debug, Copy, Clone)]
pub struct AwsIoTSettings {
    pub shadow_update: [u8; 128],
    pub shadow_delta: [u8; 128],
    pub shadow_documents: [u8; 128],
    pub device_id: [u8; 32],
}

#[derive(Debug)]
pub struct AwsIoTCertificates {
    pub server_cert: Vec<u8>,
    pub device_cert: Vec<u8>,
    pub private_key: Vec<u8>,
    pub host_url: [u8; 128],
    pub device_id: [u8; 32],
}

impl AwsIoTSettings {
    pub fn new(partition: &str) -> Result<Self, EspError> {
        let mut settings = AwsIoTSettings {
            shadow_update: [0; 128],
            shadow_delta: [0; 128],
            shadow_documents: [0; 128],
            device_id: [0; 32],
        };

        let part = EspCustomNvsPartition::take(partition)?;

        let nvs = EspCustomNvs::new(part.clone(), "aws_settings", false)?;
        nvs.get_str("shadow_update", &mut settings.shadow_update)?;
        nvs.get_str("shadow_delta", &mut settings.shadow_delta)?;
        nvs.get_str("shadow_doc", &mut settings.shadow_documents)?;

        let nvs = EspCustomNvs::new(part, "device_data", false)?;
        nvs.get_str("device_id", &mut settings.device_id)?;

        Ok(settings)
    }
}

impl AwsIoTCertificates {
    pub fn new(partition: &str) -> Result<Self, EspError> {
        let mut settings = AwsIoTCertificates {
            server_cert: Vec::new(),
            device_cert: Vec::new(),
            private_key: Vec::new(),
            host_url: [0; 128],
            device_id: [0; 32],
        };

        let part = EspCustomNvsPartition::take(partition)?;
        let nvs = EspCustomNvs::new(part.clone(), "certificates", false)?;

        if let Some(l) = nvs.len_str("server_cert")? {
            // we need 1 byte more as the certificate needs to be
            // a 0 terminated string
            match settings.server_cert.try_reserve(l + 2) {
                Ok(_) => {
                    for _ in 1..l + 2 {
                        settings.server_cert.push(0);
                    }
                    nvs.get_str("server_cert", &mut settings.server_cert[..])?;
                }
                Err(err) => {
                    panic!("Failed to reserve emory for server certificate: {err}");
                }
            }
        } else {
            panic!("server_cert in nvs not found");
        }

        if let Some(l) = nvs.len_str("device_cert")? {
            match settings.device_cert.try_reserve(l + 2) {
                Ok(_) => {
                    for _ in 1..l + 2 {
                        settings.device_cert.push(0);
                    }
                    nvs.get_str("device_cert", &mut settings.device_cert[..])?;
                }
                Err(err) => {
                    panic!("Failed to reserve emory for device certificate: {err}");
                }
            }
        } else {
            panic!("device cert in nvs not found");
        }

        if let Some(l) = nvs.len_str("priv_key")? {
            match settings.private_key.try_reserve(l + 2) {
                Ok(_) => {
                    for _ in 1..l + 2 {
                        settings.private_key.push(0);
                    }
                    nvs.get_str("priv_key", &mut settings.private_key[..])?;
                }
                Err(err) => {
                    panic!("Failed to reserve emory for private key: {err}");
                }
            }
        } else {
            panic!("private key in nvs not found");
        }

        let nvs = EspCustomNvs::new(part.clone(), "aws_settings", false)?;
        nvs.get_str("host_url", &mut settings.host_url)?;

        let nvs = EspCustomNvs::new(part, "device_data", false)?;
        nvs.get_str("device_id", &mut settings.device_id)?;

        Ok(settings)
    }
}
