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
        settings.port = u16::from_str_radix(&s[..s.len() - 2], 10).unwrap();

        let nvs = EspCustomNvs::new(part, "device_data", false)?;
        nvs.get_str("device_id", &mut settings.device_id)?;

        Ok(settings)
    }
}
