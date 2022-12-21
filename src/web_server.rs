pub mod url_handler {
    use crate::anemometer::anemometer::GLOBAL_ANEMOMETER_DATA;
    use embedded_svc::{http::server::Request, io::Write, utils::http::Headers};
    use esp_idf_svc::http::server::EspHttpConnection;
    use log::info;
    const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");
    const OTA_PAGE: &str = include_str!("../html/ota-update.html");

    pub fn ota_update_handler(
        mut req: Request<&mut EspHttpConnection>,
    ) -> embedded_svc::http::server::HandlerResult {
        use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
        use esp_idf_svc::ota::EspOta;

        const BUF_MAX: usize = 2 * 1024;
        let mut firmware_update_ok = false;

        info!("Start processing ota_update_handler /api/ota");

        let mut content_length: usize = 0;
        let mut body: [u8; BUF_MAX] = [0; BUF_MAX];
        let mut headers = Headers::<1>::new();
        headers.set_cache_control("no-store");

        let res = req.connection().read(&mut body);
        info!("POST body size: {}", res.unwrap());

        // TODO: check error handling!
        let firmware_url = url::form_urlencoded::parse(&body)
            .filter(|p| p.0 == "firmware")
            .map(|p| p.1)
            .next()
            .ok_or_else(|| anyhow::anyhow!("No parameter firmware"));

        let firmware_url = firmware_url.unwrap().to_string();
        let firmware_url = firmware_url.trim_matches(char::from(0));
        info!("Will use firmware from: {}", firmware_url);

        let mut ota = EspOta::new().expect("EspOta::new should have been successfull");

        let ota_update = ota
            .initiate_update()
            .expect("initiate ota update should have worked");

        let mut client = EspHttpConnection::new(&Configuration {
            buffer_size: Some(BUF_MAX),
            ..Default::default()
        })
        .expect("creation of EspHttpConnection should have worked");

        info!("EspHttpConnection created");
        let _resp = client.initiate_request(embedded_svc::http::Method::Get, firmware_url, &[]);

        info!("after client.initiate_request()");

        client.initiate_response()?;

        if let Some(len) = client.header("Content-Length") {
            content_length = len.parse().unwrap();
        } else {
            info!("reading content length for firmware update http request failed");
        }

        info!("Content-length: {:?}", content_length);

        info!(">>>>>>>>>>>>>>>> initiating OTA update");

        let mut bytes_read_total = 0;

        loop {
            esp_idf_hal::delay::FreeRtos::delay_ms(10);
            let n_bytes_read = match client.read(&mut body) {
                Ok(n) => n,
                Err(err) => {
                    info!("ERROR reading firmware batch {:?}", err);
                    break;
                }
            };
            bytes_read_total += n_bytes_read;

            if !body.is_empty() {
                match ota_update.write(&body) {
                    Ok(_) => {}
                    Err(err) => {
                        info!("ERROR failed to write update with: {:?}", err);
                        break;
                    }
                }
            } else {
                info!("ERROR firmware image with zero length");
                break;
            }

            if body.len() > n_bytes_read {
                break;
            }
        }

        if bytes_read_total == content_length {
            firmware_update_ok = true;
        }

        let confirmation_msg = if firmware_update_ok {
            ota_update.complete().unwrap();
            info!("completed firmware update");

            templated("Successfully completed firmware update")
        } else {
            ota_update.abort().unwrap();
            info!("ERROR firmware update failed");
            templated("Firmare update failed")
        };

        let mut response = req.into_response(200, None, headers.as_slice())?;
        response.write_all(confirmation_msg.as_bytes())?;

        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        info!("restarting device after firmware update");
        unsafe {
            esp_idf_sys::esp_restart();
        }
    }

    pub fn api_version_handler(
        req: Request<&mut EspHttpConnection>,
    ) -> embedded_svc::http::server::HandlerResult {
        let mut headers = Headers::<1>::new();
        headers.set_cache_control("no-store");

        let mut resp = req.into_response(200, None, headers.as_slice())?;
        resp.write_all(FIRMWARE_VERSION.as_bytes())?;
        info!("Processing '/api/version' request");

        Ok(())
    }

    pub fn home_page_handler(
        req: Request<&mut EspHttpConnection>,
    ) -> embedded_svc::http::server::HandlerResult {
        let mut headers = Headers::<1>::new();
        headers.set_cache_control("no-store");

        let mut response = req.into_response(200, None, headers.as_slice())?;
        response.write_all(OTA_PAGE.as_bytes())?;
        info!("Processing '/' request");

        Ok(())
    }

    pub fn windspeed_handler(
        req: Request<&mut EspHttpConnection>,
    ) -> embedded_svc::http::server::HandlerResult {
        let html = windspeed(GLOBAL_ANEMOMETER_DATA.lock().unwrap().rps);
        let mut headers = Headers::<1>::new();
        headers.set_cache_control("no-store");

        let mut resp = req.into_response(200, None, headers.as_slice())?;
        resp.write_all(html.as_bytes())?;
        info!("Processing '/windspeed' request");
        Ok(())
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

    fn windspeed(val: f32) -> String {
        templated(format!("Rotation speed: {:.2} rps", val))
    }
}
