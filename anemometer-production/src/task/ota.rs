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
use crate::state::*;
use crate::utils::{aws_credential_service::*, errors::*};
use core::mem;
use core::ptr;
use embassy_time::{Duration, Timer};
use embedded_svc::ota::{FirmwareInfo, FirmwareInfoLoader, LoadResult, Slot};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::ota::{EspFirmwareInfoLoader, EspOta};
use esp_idf_sys::*;
use heapless::String;
use log::*;

const WRITE_DATA_BUF_SIZE: usize = 8196;
const TX_BUF_SIZE: usize = 4096;

pub async fn ota_task(aws_certificates: &'static AwsIoTCertificates) {
    let mut subscriber = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();
    info!("OTA Task Started");
    let mut s3_url: std::string::String = std::string::String::new();
    let mut s3_fw_bucket: std::string::String = std::string::String::new();
    let mut aws_region: std::string::String = std::string::String::new();

    {
        let aws_config = super::super::AWSCONFIG.lock().unwrap();

        s3_url.push_str(&aws_config.s3_url);
        s3_fw_bucket.push_str(&aws_config.s3_fw_bucket);
        aws_region.push_str(&aws_config.region);
    }

    loop {
        if let ApplicationStateChange::OTAUpdateRequest(firmware_file_name) =
            subscriber.next_message_pure().await
        {
            info!("processing OTA request for {}", firmware_file_name);

            let publisher = APPLICATION_EVENT_CHANNEL.publisher().unwrap();

            // Notify all tasks that the OTA update started. These tasks are
            // expected to shutdown
            let data = ApplicationStateChange::OTAUpdateStarted;
            publisher.publish(data).await;
            Timer::after(Duration::from_secs(2)).await;

            if let Err(err) = perform_update(
                &s3_url,
                &aws_region,
                &s3_fw_bucket,
                &firmware_file_name,
                aws_certificates,
            ) {
                error!("Firmware update failed: {err}");
            } else {
                info!("Firmware update successful. Restarting device.");
            }

            esp_idf_hal::delay::FreeRtos::delay_ms(5000);
            unsafe {
                esp_idf_sys::esp_restart();
            }
        }
    }
}

// TODO: as of Dec 2022 there is no async http client implementation for ESP IDF.
// once an async implementation becomes available rework this code to become async
fn perform_update(
    aws_s3_url: &str,
    aws_region: &str,
    aws_bucket: &str,
    firmware_file_name: &str,
    aws_certificates: &'static AwsIoTCertificates,
) -> Result<(), OtaError> {
    let content_length: usize;
    let mut ota_write_data: [u8; WRITE_DATA_BUF_SIZE] = [0; WRITE_DATA_BUF_SIZE];
    let mut invalid_fw_version: heapless::String<32> = String::new();
    let mut found_invalid_fw = false;
    let mut update_summary: heapless::String<410> = String::new();

    let aws_credentials = Credentials::new(aws_certificates).unwrap();

    let firmware_url = signe_url(
        aws_credentials,
        aws_s3_url,
        aws_region,
        aws_bucket,
        firmware_file_name,
    )
    .unwrap();

    let mut client = EspHttpConnection::new(&Configuration {
        buffer_size: Some(WRITE_DATA_BUF_SIZE),
        buffer_size_tx: Some(TX_BUF_SIZE),
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })
    .expect("creation of EspHttpConnection should have worked");

    let _resp = match client.initiate_request(
        embedded_svc::http::Method::Get,
        firmware_url.as_str(),
        &[],
    ) {
        Ok(c) => c,
        Err(err) => {
            error!("Failed to initiate request {}", err);
        }
    };

    if let Err(err) = client.initiate_response() {
        error!("Error initiate response {}", err);
        return Err(OtaError::HttpError);
    }

    let http_status = client.status();
    if http_status != 200 {
        error!("download fw image failed. Server response = {http_status}");
        return Err(OtaError::FwImageNotFound);
    }

    if let Some(len) = client.header("Content-Length") {
        content_length = len.parse().unwrap();
    } else {
        error!("reading content length for firmware update http request failed");
        return Err(OtaError::FwImageNotFound);
    }
    if content_length < WRITE_DATA_BUF_SIZE {
        error!("Error content-length too short. Length = {content_length}");
        return Err(OtaError::FwImageNotFound);
    }

    info!("initiating OTA update");

    let update_partition: esp_partition_t =
        unsafe { *esp_ota_get_next_update_partition(ptr::null()) };
    let partition_label =
        std::str::from_utf8(unsafe { std::mem::transmute(&update_partition.label as &[i8]) })
            .unwrap()
            .trim_matches(char::from(0));
    info!(
        "Writing to partition {} subtype {:#4x} size {:#10x} at offset {:#10x}",
        partition_label, update_partition.subtype, update_partition.size, update_partition.address
    );

    let mut ota = EspOta::new().expect("EspOta::new should have been successfull");

    let boot_slot = ota.get_boot_slot().unwrap();
    let run_slot = ota.get_running_slot().unwrap();
    let update_slot = ota.get_update_slot().unwrap();

    if let Some(slot) = ota.get_last_invalid_slot().unwrap() {
        info!("last invalid slot = {:?}", slot);
        if slot.firmware.is_some() {
            let fw = slot.firmware.unwrap();
            if let Err(err) = invalid_fw_version.push_str(fw.version.as_str()) {
                error!("failed to load invalid fw version {:?}", err);
            }
            found_invalid_fw = true;
        }
    } else {
        info!("no invalid slot found");
    }

    let ota_update = match ota.initiate_update() {
        Ok(handle) => handle,
        Err(_) => return Err(OtaError::OtaApiError),
    };

    let mut bytes_read_total = 0;
    let mut image_header_was_checked = false;

    loop {
        let data_read = match client.read(&mut ota_write_data) {
            Ok(n) => n,
            Err(err) => {
                error!("ERROR reading firmware batch {:?}", err);
                return Err(OtaError::HttpError);
            }
        };

        // Check if first segment and process image meta data
        if !image_header_was_checked
            && data_read
                > mem::size_of::<esp_image_header_t>()
                    + mem::size_of::<esp_image_segment_header_t>()
                    + mem::size_of::<esp_app_desc_t>()
        {
            let mut esp_fw_loader_info = EspFirmwareInfoLoader::new();
            let res = match esp_fw_loader_info.load(&ota_write_data) {
                Ok(load_result) => load_result,
                Err(err) => {
                    error!("failed to retrive firmware info from download: {err}");
                    return Err(OtaError::OtaApiError);
                }
            };
            if res != LoadResult::Loaded {
                error!("incomplete data for retriving FW info for downloaded FW");
                return Err(OtaError::OtaApiError);
            }

            let fw_info = esp_fw_loader_info.get_info().unwrap();

            format_update_summary(
                &mut update_summary,
                boot_slot.clone(),
                run_slot.clone(),
                update_slot.clone(),
                fw_info.clone(),
            );
            info!("\n{update_summary}\n");

            if found_invalid_fw && invalid_fw_version == fw_info.version {
                info!("New FW has same version as invalide firmware slot. Stopping update");
                return Err(OtaError::FwSameAsInvalidFw);
            }

            image_header_was_checked = true;
        }

        bytes_read_total += data_read;

        if data_read > 0 {
            if let Err(err) = ota_update.write(&ota_write_data) {
                error!("ERROR failed to write update with: {err:?}");
                return Err(OtaError::FlashFailed);
            }
        }

        // Check if we have read an entire buffer. If not,
        // we assume it was the last segment and we stop
        if ota_write_data.len() > data_read {
            break;
        }
    }

    if bytes_read_total == content_length {
        if let Err(err) = ota_update.complete() {
            error!("OTA update failed. esp_ota_end failed {:?}", err);
            return Err(OtaError::OtaApiError);
        }
    } else {
        ota_update.abort().unwrap();
        error!("ERROR firmware update failed");
        return Err(OtaError::ImageLoadIncomplete);
    };

    Ok(())
}

fn format_update_summary<const N: usize>(
    update_summary: &mut heapless::String<N>,
    boot_slot: Slot,
    run_slot: Slot,
    update_slot: Slot,
    ota_image_info: FirmwareInfo,
) {
    let mut label: heapless::String<10> = heapless::String::new();

    update_summary.push_str("OTA Update Summary\n").unwrap();
    update_summary.push_str("==================\n").unwrap();
    update_summary.push_str("Boot   partition: ").unwrap();
    copy_truncated_string(&mut label, boot_slot.label);
    update_summary.push_str(label.as_str()).unwrap();
    update_summary.push_str(", ").unwrap();
    add_firmware_info(update_summary, boot_slot.firmware);

    update_summary.push_str("\nRun    partition: ").unwrap();
    label = heapless::String::new();
    copy_truncated_string(&mut label, run_slot.label);
    update_summary.push_str(label.as_str()).unwrap();
    update_summary.push_str(", ").unwrap();
    add_firmware_info(update_summary, run_slot.firmware);

    update_summary.push_str("\nUpdate partition: ").unwrap();
    label = heapless::String::new();
    copy_truncated_string(&mut label, update_slot.label);
    update_summary.push_str(label.as_str()).unwrap();
    update_summary.push_str(", ").unwrap();
    add_firmware_info(update_summary, update_slot.firmware);
    update_summary.push_str("\n").unwrap();

    update_summary.push_str("\nDownloaded FW  : ").unwrap();
    add_firmware_info(update_summary, Some(ota_image_info));
    update_summary.push_str("\n").unwrap();
}

fn add_firmware_info<const N: usize>(
    update_summary: &mut heapless::String<N>,
    firmware: Option<FirmwareInfo>,
) {
    let mut version: heapless::String<10> = heapless::String::new();
    let mut released: heapless::String<19> = heapless::String::new();
    let mut description: heapless::String<32> = heapless::String::new();

    if let Some(fw) = firmware {
        copy_truncated_string(&mut version, fw.version);
        update_summary.push_str(version.as_str()).unwrap();
        update_summary.push_str(", ").unwrap();
        copy_truncated_string(&mut released, fw.released);
        update_summary.push_str(released.as_str()).unwrap();
        if let Some(desc) = fw.description {
            update_summary.push_str(", ").unwrap();
            copy_truncated_string(&mut description, desc);
            update_summary.push_str(description.as_str()).unwrap();
        }
    }
}

fn copy_truncated_string<const N: usize, const M: usize>(
    dest: &mut heapless::String<N>,
    src: heapless::String<M>,
) {
    src.as_str()
        .chars()
        .enumerate()
        .take_while(|c| c.0 < N)
        .for_each(|c| dest.push(c.1).unwrap());
}
