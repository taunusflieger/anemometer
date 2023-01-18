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
use log::*;

#[derive(Debug, Copy, Clone)]
pub struct AwsIoTSettings {
    pub things_prefix: [u8; 24],
    pub shadow_update_postfix: [u8; 128],
    pub shadow_delta_postfix: [u8; 128],
    pub shadow_documents_postfix: [u8; 128],
    pub device_id: [u8; 32],
    pub topic_prefix: [u8; 128],
    pub cmd_topic_postfix: [u8; 128],
    pub region: [u8; 32],
    pub s3_url: [u8; 128],
    pub s3_fw_bucket: [u8; 32],
    pub credential_provider_endpoint: [u8; 180],
}

#[derive(Debug)]
pub struct AwsIoTCertificates {
    pub device_cert: Vec<u8>,
    pub private_key: Vec<u8>,
    pub mqtt_endpoint: [u8; 128],
    pub device_id: [u8; 32],
}

impl AwsIoTSettings {
    pub fn new(partition: &str) -> Result<Self, EspError> {
        let mut settings = AwsIoTSettings {
            things_prefix: [0; 24],
            shadow_update_postfix: [0; 128],
            shadow_delta_postfix: [0; 128],
            shadow_documents_postfix: [0; 128],
            device_id: [0; 32],
            topic_prefix: [0; 128],
            cmd_topic_postfix: [0; 128],
            region: [0; 32],
            s3_url: [0; 128],
            s3_fw_bucket: [0; 32],
            credential_provider_endpoint: [0; 180],
        };
        info!("Loading AwsIoTSettings");

        let part = EspCustomNvsPartition::take(partition)?;

        let nvs = EspCustomNvs::new(part.clone(), "aws_settings", false)?;
        if let Err(err) = nvs.get_str("things_prefix", &mut settings.things_prefix) {
            error!("failed to load config data things_prefix: {}", err);
        }
        if let Err(err) = nvs.get_str("shadow_update", &mut settings.shadow_update_postfix) {
            error!("failed to load config data shadow_update_postfix: {}", err);
        }
        if let Err(err) = nvs.get_str("shadow_delta", &mut settings.shadow_delta_postfix) {
            error!("failed to load config data shadow_delta_postfix: {}", err);
        }
        if let Err(err) = nvs.get_str("shadow_doc", &mut settings.shadow_documents_postfix) {
            error!("failed to load config data shadow_doc_postfix: {}", err);
        }
        if let Err(err) = nvs.get_str("topic_prefix", &mut settings.topic_prefix) {
            error!("failed to load config data topic_prefix: {}", err);
        }
        if let Err(err) = nvs.get_str("cmd_topic", &mut settings.cmd_topic_postfix) {
            error!("failed to load config data cmd_topic_postfix: {}", err);
        }
        if let Err(err) = nvs.get_str("region", &mut settings.region) {
            error!("failed to load config data region: {}", err);
        }
        if let Err(err) = nvs.get_str("s3_url", &mut settings.s3_url) {
            error!("failed to load config data s3_url: {}", err);
        }
        if let Err(err) = nvs.get_str("s3_fw_bucket", &mut settings.s3_fw_bucket) {
            error!("failed to load config data s3_fw_bucket: {}", err);
        }
        if let Err(err) = nvs.get_str("cred_prov_ep", &mut settings.credential_provider_endpoint) {
            error!("failed to load config data credent_prov_endp: {}", err);
        }

        let nvs = EspCustomNvs::new(part, "device_data", false)?;
        if let Err(err) = nvs.get_str("device_id", &mut settings.device_id) {
            error!("failed to load config data device_id: {}", err);
        }

        Ok(settings)
    }
}

impl AwsIoTCertificates {
    pub fn new(partition: &str) -> Result<Self, EspError> {
        let mut settings = AwsIoTCertificates {
            device_cert: Vec::new(),
            private_key: Vec::new(),
            mqtt_endpoint: [0; 128],
            device_id: [0; 32],
        };
        info!("Loading AwsIoTCertificates");
        let part = EspCustomNvsPartition::take(partition)?;
        let nvs = EspCustomNvs::new(part.clone(), "certificates", false)?;

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
        if let Err(err) = nvs.get_str("mqtt_endpoint", &mut settings.mqtt_endpoint) {
            error!("failed to load config data mqtt_endpoint: {}", err);
        }

        let nvs = EspCustomNvs::new(part, "device_data", false)?;
        if let Err(err) = nvs.get_str("device_id", &mut settings.device_id) {
            error!("failed to load config data device_id: {}", err);
        }

        Ok(settings)
    }
}
