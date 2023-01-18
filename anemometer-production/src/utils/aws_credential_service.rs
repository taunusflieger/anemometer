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
use crate::configuration::AwsIoTCertificates;
use crate::utils::errors::*;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use log::*;
use rusty_s3::{Bucket, S3Action, UrlStyle};
use serde::{Deserialize, Serialize};

use std::time::Duration;
// The function in this module will be called only in case of a SW update which
// will result in a restart of the device, so we use the heap.

const WRITE_DATA_BUF_SIZE: usize = 1024;
// Lifetime of the generated AWS token
const AWS_TOKEN_LIFETIME: u64 = 60 * 10;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub credentials: Credentials,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: String,
}

impl Credentials {
    pub fn new(aws_certificates: &'static AwsIoTCertificates) -> Result<Self, AwsError> {
        let content_length: usize;
        let mut access_key_buffer: [u8; WRITE_DATA_BUF_SIZE] = [0; WRITE_DATA_BUF_SIZE];
        let mut credential_provider_endpoint: String = String::new();

        {
            let aws_config = super::super::AWSCONFIG.lock().unwrap();
            // need to remove tailing zeros otherwise CString will complain
            credential_provider_endpoint.push_str(
                core::str::from_utf8(
                    &(aws_config.credential_provider_endpoint[0..aws_config
                        .credential_provider_endpoint
                        .iter()
                        .position(|&x| x == 0)
                        .unwrap()]),
                )
                .unwrap(),
            );
        }

        let x509_client_cert =
            esp_idf_svc::tls::X509::pem_until_nul(&aws_certificates.device_cert[..]);
        let x509_client_priv_key =
            esp_idf_svc::tls::X509::pem_until_nul(&aws_certificates.private_key[..]);

        let mut client = EspHttpConnection::new(&Configuration {
            buffer_size: Some(WRITE_DATA_BUF_SIZE),
            client_certificate: Some(x509_client_cert),
            private_key: Some(x509_client_priv_key),
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            ..Default::default()
        })
        .expect("Failed to create HttpConnection for AWS credential provider");

        let _resp = client.initiate_request(
            embedded_svc::http::Method::Get,
            &credential_provider_endpoint,
            &[],
        );

        if let Err(err) = client.initiate_response() {
            error!("Error initiate response {}", err);
            return Err(AwsError::AwsCredentialsError);
        }

        let http_status = client.status();
        if http_status != 200 {
            error!("download AWS credentials failed. Server response = {http_status}");
            return Err(AwsError::AwsCredentialsError);
        }

        if let Some(len) = client.header("Content-Length") {
            content_length = len.parse().unwrap();
        } else {
            error!("reading content length for AWS credentials http request failed");
            return Err(AwsError::AwsCredentialsError);
        }
        if content_length < WRITE_DATA_BUF_SIZE {
            error!("Error content-length too short. Length = {content_length}");
            return Err(AwsError::AwsCredentialsError);
        }

        let mut access_keys: String = String::new();

        loop {
            let data_read = match client.read(&mut access_key_buffer) {
                Ok(n) => n,
                Err(err) => {
                    error!("ERROR reading firmware batch {:?}", err);
                    return Err(AwsError::AwsCredentialsError);
                }
            };

            if data_read > 0 {
                access_keys
                    .push_str(core::str::from_utf8(&access_key_buffer[0..data_read]).unwrap());
            }

            // Check if we have read an entire buffer. If not,
            // we assume it was the last segment and we stop
            if access_key_buffer.len() > data_read {
                break;
            }
        }

        let deserialized: Root = match serde_json::from_str(&access_keys) {
            Ok(d) => d,
            Err(err) => {
                error!(
                    "ERROR deserializing AWS credential provider response {:?}",
                    err
                );
                return Err(AwsError::AwsCredentialsError);
            }
        };

        Ok(deserialized.credentials)
    }
}

pub fn signe_url(
    aws_credentials: Credentials,
    endpoint: &str,
    region: &str,
    bucket_name: &str,
    fw_file_name: &str,
) -> Result<String, AwsError> {
    let bucket = Bucket::new(
        endpoint.to_string().parse().unwrap(),
        UrlStyle::VirtualHost,
        bucket_name.to_string(),
        region.to_string(),
    )
    .expect("Url has a valid scheme and host");

    let credentials = rusty_s3::Credentials::new_with_token(
        aws_credentials.access_key_id,
        aws_credentials.secret_access_key,
        aws_credentials.session_token,
    );
    // signing a request
    let presigned_url_duration = Duration::from_secs(AWS_TOKEN_LIFETIME);
    let action = bucket.get_object(Some(&credentials), fw_file_name);
    let signed_url = action.sign(presigned_url_duration);

    Ok(signed_url.to_string())
}
