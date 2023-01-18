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
const AWS_CREDENTIAL_PROVIDER_ENPOINT: &str =
    "https://c2syniqfqg2c95.credentials.iot.eu-west-1.amazonaws.com/role-aliases/WeatherStationInstrument-s3-access-role-alias/credentials";

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

        info!("EspHttpConnection created");
        let _resp = client.initiate_request(
            embedded_svc::http::Method::Get,
            AWS_CREDENTIAL_PROVIDER_ENPOINT,
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

        info!("Content-length: {:?}", content_length);

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
        info!("Credential Provider response = {}", access_keys);

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
        info!("Credentials: {:?}", deserialized.credentials);

        Ok(deserialized.credentials)
    }
}

//
// signe_url(.., "https://s3.eu-west-1.amazonaws.com", "eu-west-1", "anemometer-fw-store", "firmware-0.1.2.bin" )
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

    info!("bucket = {:?}", bucket);

    let credentials = rusty_s3::Credentials::new_with_token(
        aws_credentials.access_key_id,
        aws_credentials.secret_access_key,
        aws_credentials.session_token,
    );
    // signing a request
    let presigned_url_duration = Duration::from_secs(AWS_TOKEN_LIFETIME);
    let action = bucket.get_object(Some(&credentials), fw_file_name);
    let signed_url = action.sign(presigned_url_duration);
    info!("curl '{}'", signed_url);
    Ok(signed_url.to_string())
}
