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
use crate::utils::nvs_ext::*;
use esp_idf_svc::nvs::*;
use esp_idf_sys::*;
use log::*;

const NVS_STRING_READ_BUFFER_SIZE: usize = 180;

#[derive(Debug)]
pub struct AwsIoTSettings {
    pub things_prefix: String,
    pub shadow_update_postfix: String,
    pub shadow_delta_postfix: String,
    pub shadow_documents_postfix: String,
    pub device_id: String,
    pub topic_prefix: String,
    pub cmd_topic_postfix: String,
    pub region: String,
    pub s3_url: String,
    pub s3_fw_bucket: String,
    pub credential_provider_endpoint: String,
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
        let part = EspCustomNvsPartition::take(partition)?;

        let nvs = EspCustomNvs::new(part.clone(), "device_data", false)?;

        let mut v: u8 = 0;
        nvs.get_u8("test_u8", &mut v)?;
        info!("v = {}", v);

        let mut v: i8 = 0;
        nvs.get_i8("test_i8", &mut v)?;
        info!("v = {}", v);

        let mut v: u16 = 0;
        nvs.get_u16("test_u16", &mut v)?;
        info!("v = {}", v);

        let mut v: i16 = 0;
        nvs.get_i16("test_i16", &mut v)?;
        info!("v = {}", v);

        let mut v: u32 = 0;
        nvs.get_u32("test_u32", &mut v)?;
        info!("v = {}", v);

        let mut v: i32 = 0;
        nvs.get_i32("test_i32", &mut v)?;
        info!("v = {}", v);

        let mut v: u64 = 0;
        nvs.get_u64("test_u64", &mut v)?;
        info!("v = {}", v);

        let mut v: i64 = 0;
        nvs.get_i64("test_i64", &mut v)?;
        info!("v = {}", v);

        let nvs = EspCustomNvs::new(part.clone(), "aws_settings", false)?;

        Ok(AwsIoTSettings {
            things_prefix: get_string_from_nvs(&nvs, "things_prefix")?,
            shadow_update_postfix: get_string_from_nvs(&nvs, "shadow_update")?,
            shadow_delta_postfix: get_string_from_nvs(&nvs, "shadow_delta")?,
            shadow_documents_postfix: get_string_from_nvs(&nvs, "shadow_doc")?,
            topic_prefix: get_string_from_nvs(&nvs, "topic_prefix")?,
            cmd_topic_postfix: get_string_from_nvs(&nvs, "cmd_topic")?,
            region: get_string_from_nvs(&nvs, "region")?,
            s3_url: get_string_from_nvs(&nvs, "s3_url")?,
            s3_fw_bucket: get_string_from_nvs(&nvs, "s3_fw_bucket")?,
            credential_provider_endpoint: get_string_from_nvs(&nvs, "cred_prov_ep")?,
            device_id: {
                let nvs = EspCustomNvs::new(part, "device_data", false)?;
                match get_string_from_nvs(&nvs, "device_id") {
                    Ok(s) => s,
                    Err(err) => return Err(err),
                }
            },
        })
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
        nvs.get_str("mqtt_endpoint", &mut settings.mqtt_endpoint)?;

        let nvs = EspCustomNvs::new(part, "device_data", false)?;
        nvs.get_str("device_id", &mut settings.device_id)?;

        Ok(settings)
    }
}

fn get_string_from_nvs(nvs: &EspCustomNvs, key: &str) -> Result<String, EspError> {
    let mut nvm_str_buffer: [u8; NVS_STRING_READ_BUFFER_SIZE] = [0; NVS_STRING_READ_BUFFER_SIZE];
    nvs.get_str(key, &mut nvm_str_buffer)?;

    // remove any tailing zeros
    Ok(String::from(
        core::str::from_utf8(
            &(nvm_str_buffer[0..nvm_str_buffer.iter().position(|&x| x == 0).unwrap()]),
        )
        .unwrap(),
    ))
}
