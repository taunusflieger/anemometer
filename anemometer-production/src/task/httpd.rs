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
use crate::services::http::*;
use crate::state::*;
use embassy_futures::select::{select, Either};
use log::*;

pub async fn http_server_task() {
    // use channel_bridge::asynch::*;
    use embedded_svc::io::blocking::Write;
    use embedded_svc::utils::http::Headers;
    use esp_idf_svc::http::server::Configuration;

    const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

    let mut network_event = NETWORK_EVENT_CHANNEL.subscriber().unwrap();
    let mut app_event = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();

    let httpd = LazyInitHttpServer::new();

    loop {
        // We are interested in network events (wifi disconnected or
        // IP address assigned), or OTA update started. On all of
        // those events we need to react.
        let (network_state, app_state) = match select(
            network_event.next_message_pure(),
            app_event.next_message_pure(),
        )
        .await
        {
            Either::First(network_state) => {
                info!("network change event received");
                (Some(network_state), None)
            }
            Either::Second(app_state) => {
                info!("app state change event received");
                (None, Some(app_state))
            }
        };

        if let Some(ApplicationStateChange::OTAUpdateStarted) = app_state {
            info!("OTA Update started shutting down http server");
            httpd.clear();
            break;
        }

        match network_state {
            Some(NetworkStateChange::IpAddressAssigned { ip }) => {
                let conf = Configuration::default();
                let mut s = httpd.create(&conf);

                info!("http_server_task: starting httpd on address: {:?}", ip);
                if let Err(err) = s.fn_handler("/", embedded_svc::http::Method::Get, move |req| {
                    let mut avg_speed = 0.0;
                    let mut wind_gust = 0.0;
                    let mut headers = Headers::<1>::new();
                    headers.set_cache_control("no-store");

                    if let Ok(wind_historian) = (*WIND_DATA_HISTORY).lock() {
                        avg_speed = wind_historian.avg_speed();
                        wind_gust = wind_historian.gust_speed();
                    };
                    let html = windspeed(avg_speed, wind_gust);

                    let mut resp = req.into_response(200, None, headers.as_slice())?;
                    resp.write_all(html.as_bytes())?;

                    info!("Processing '/' request");
                    Ok(())
                }) {
                    info!(
                        "http_server_task: failed to register http handler /: {:?} - restarting device",
                        err
                    );
                    unsafe {
                        esp_idf_sys::esp_restart();
                    }
                }
            }
            Some(NetworkStateChange::WifiDisconnected) => {
                info!("http_server_task: stopping httpd");
                httpd.clear();
            }
            None => {}
        }
    }
    info!("http_server_task shutdown");
}

fn templated(content: impl AsRef<str>) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>esp-rs web server</title>
</head>
<body>
    {}
</body>
</html>
"#,
        content.as_ref()
    )
}

fn windspeed(speed: f32, gust: f32) -> String {
    templated(format!(
        "Wind speed: {:.2} km/h\nWind gust: {:.2} km/h",
        speed, gust
    ))
}
