#![allow(dead_code)]
pub mod gps {
    use crate::gpio::*;
    use esp_idf_hal::delay::BLOCK;
    use esp_idf_hal::gpio;
    use esp_idf_hal::peripheral::Peripheral;
    use esp_idf_hal::uart::{config, Uart, UartDriver};
    use esp_idf_hal::units::*;
    use esp_idf_sys::*;
    use log::info;

    pub const PMTK_SET_NMEA_UPDATE_100_MILLIHERTZ: &str = "$PMTK220,10000*2F";
    ///< Once every 10 seconds, 100 millihertz.
    pub const PMTK_SET_NMEA_UPDATE_200_MILLIHERTZ: &str = "$PMTK220,5000*1B";
    ///< Once every 5 seconds, 200 millihertz.
    pub const PMTK_SET_NMEA_UPDATE_1HZ: &str = "$PMTK220,1000*1F";
    ///<  1 Hz
    pub const PMTK_SET_NMEA_UPDATE_2HZ: &str = "$PMTK220,500*2B";
    ///<  2 Hz
    pub const PMTK_SET_NMEA_UPDATE_5HZ: &str = "$PMTK220,200*2C";
    ///<  5 Hz
    pub const PMTK_SET_NMEA_UPDATE_10HZ: &str = "$PMTK220,100*2F";
    ///< 10 Hz
    // Position fix update rate commands.
    pub const PMTK_API_SET_FIX_CTL_100_MILLIHERTZ: &str = "$PMTK300,10000,0,0,0,0*2C";
    ///< Once every 10 seconds, 100 millihertz.
    pub const PMTK_API_SET_FIX_CTL_200_MILLIHERTZ: &str = "$PMTK300,5000,0,0,0,0*18";
    ///< Once every 5 seconds, 200 millihertz.
    pub const PMTK_API_SET_FIX_CTL_1HZ: &str = "$PMTK300,1000,0,0,0,0*1C";
    ///< 1 Hz
    pub const PMTK_API_SET_FIX_CTL_5HZ: &str = "$PMTK300,200,0,0,0,0*2F";
    ///< 5 Hz
    // Can't fix position faster than 5 times a second!

    pub const PMTK_SET_BAUD_115200: &str = "$PMTK251,115200*1F";
    ///< 115200 bps
    pub const PMTK_SET_BAUD_57600: &str = "$PMTK251,57600*2C";
    ///<  57600 bps
    pub const PMTK_SET_BAUD_9600: &str = "$PMTK251,9600*17";
    ///<   9600 bps

    pub const PMTK_SET_NMEA_OUTPUT_GLLONLY: &str =
        "$PMTK314,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*29";
    ///< turn on only the
    ///< GPGLL sentence
    pub const PMTK_SET_NMEA_OUTPUT_RMCONLY: &str =
        "$PMTK314,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*29";
    ///< turn on only the
    ///< GPRMC sentence
    pub const PMTK_SET_NMEA_OUTPUT_VTGONLY: &str =
        "$PMTK314,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*29";
    ///< turn on only the
    ///< GPVTG
    pub const PMTK_SET_NMEA_OUTPUT_GGAONLY: &str =
        "$PMTK314,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*29";
    ///< turn on just the
    ///< GPGGA
    pub const PMTK_SET_NMEA_OUTPUT_GSAONLY: &str =
        "$PMTK314,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0*29";
    ///< turn on just the
    ///< GPGSA
    pub const PMTK_SET_NMEA_OUTPUT_GSVONLY: &str =
        "$PMTK314,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0*29";
    ///< turn on just the
    ///< GPGSV
    pub const PMTK_SET_NMEA_OUTPUT_RMCGGA: &str =
        "$PMTK314,0,1,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*28";
    ///< turn on GPRMC and
    ///< GPGGA
    pub const PMTK_SET_NMEA_OUTPUT_RMCGGAGSA: &str =
        "$PMTK314,0,1,0,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0*29";
    ///< turn on GPRMC, GPGGA
    ///< and GPGSA
    pub const PMTK_SET_NMEA_OUTPUT_ALLDATA: &str =
        "$PMTK314,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0*28";
    ///< turn on ALL THE DATA
    pub const PMTK_SET_NMEA_OUTPUT_OFF: &str = "$PMTK314,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0*28";
    ///< turn off output

    // to generate your own sentences, check out the MTK command datasheet and use a
    // checksum calculator such as the awesome
    // http://www.hhhh.org/wiml/proj/nmeaxor.html

    pub const PMTK_LOCUS_STARTLOG: &str = "$PMTK185,0*22";
    ///< Start logging data
    pub const PMTK_LOCUS_STOPLOG: &str = "$PMTK185,1*23";
    ///< Stop logging data
    pub const PMTK_LOCUS_STARTSTOPACK: &str = "$PMTK001,185,3*3C";
    ///< Acknowledge the start or stop command
    pub const PMTK_LOCUS_QUERY_STATUS: &str = "$PMTK183*38";
    ///< Query the logging status
    pub const PMTK_LOCUS_ERASE_FLASH: &str = "$PMTK184,1*22";
    ///< Erase the log flash data

    pub const PMTK_ENABLE_SBAS: &str = "$PMTK313,1*2E";
    ///< Enable search for SBAS satellite (only works with 1Hz
    ///< output rate)
    pub const PMTK_ENABLE_WAAS: &str = "$PMTK301,2*2E";
    ///< Use WAAS for DGPS correction data

    pub const PMTK_STANDBY: &str = "$PMTK161,0*28";
    ///< standby command & boot successful message
    pub const PMTK_STANDBY_SUCCESS: &str = "$PMTK001,161,3*36";
    ///< Not needed currently
    pub const PMTK_AWAKE: &str = "$PMTK010,002*2D";
    ///< Wake up

    pub const PMTK_Q_RELEASE: &str = "$PMTK605*31";
    ///< ask for the release and version

    pub const PGCMD_ANTENNA: &str = "$PGCMD,33,1*6C";
    ///< request for updates on antenna status
    pub const PGCMD_NOANTENNA: &str = "$PGCMD,33,0*6D";
    ///< don'

    pub struct Mtk3339<'d> {
        pub uart: UartDriver<'d>,
        nmea_sentence: String,
    }

    impl<'d> Mtk3339<'d> {
        pub fn new<UART: Uart>(
            baud_rate: u32,
            uart: impl Peripheral<P = UART> + 'd,
            tx: impl Peripheral<P = impl OutputPin> + 'd,
            rx: impl Peripheral<P = impl InputPin> + 'd,
        ) -> Result<Mtk3339<'d>, EspError> {
            let config = config::Config::new().baudrate(Hertz(baud_rate));
            let uart_driver = UartDriver::new(
                uart,
                tx,
                rx,
                Option::<gpio::Gpio0>::None,
                Option::<gpio::Gpio1>::None,
                &config,
            )?;
            Ok(Mtk3339 {
                uart: uart_driver,
                nmea_sentence: String::new(),
            })
        }

        pub fn read_line(&mut self) -> Result<String, EspError> {
            const SENTENCE_LENGTH: usize = 120;
            let mut ch = [0_u8; 1];
            let mut buf = [0_u8; SENTENCE_LENGTH];
            let mut buf_idx: usize;
            loop {
                buf_idx = 0;

                loop {
                    self.uart.read(&mut ch, BLOCK)?;
                    while ch[0] == 0x0D || ch[0] == 0x0A {
                        self.uart.read(&mut ch, BLOCK)?;
                    }
                    if ch[0] == b'$' {
                        break;
                    }
                }

                buf[buf_idx] = ch[0];
                buf_idx += 1;
                self.uart.read(&mut ch, BLOCK)?;
                while ch[0] != 0x0D && ch[0] != 0x0A && buf_idx < SENTENCE_LENGTH {
                    buf[buf_idx] = ch[0];
                    buf_idx += 1;
                    self.uart.read(&mut ch, BLOCK)?;
                }

                break;
            }

            let s = match std::str::from_utf8(&buf[0..buf_idx]) {
                Ok(t) => t,
                Err(error) => {
                    info!("ERROR: {:?}", error);
                    ""
                }
            };

            Ok(String::from(s))
        }

        pub fn send_command(&self, cmd: &str) {
            const CRLF: [u8; 2] = [0x0D, 0x0A];
            self.uart.write(&CRLF).unwrap();
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
            self.uart.write(&CRLF).unwrap();
            esp_idf_hal::delay::FreeRtos::delay_ms(100);
            self.uart.write(cmd.as_bytes()).unwrap();
            self.uart.write(&CRLF).unwrap();
        }

        pub fn process_gps_input(input_buffer: &mut [u8]) -> Option<String> {
            const SENTENCE_BUF_SIZE: usize = 80;
            let mut sentence_buf = [0_u8; SENTENCE_BUF_SIZE];
            let mut sentence_available = false;
            let start: usize;
            let mut end: usize = 0;

            let res = input_buffer.iter().position(|&x| x == b'$');

            if res.is_some() {
                start = res.unwrap();

                // find end of the sentence
                let res = input_buffer[start..]
                    .iter()
                    .position(|&x| x == 0x0d || x == 0x0a);
                if res.is_some() {
                    end = res.unwrap();
                    if end > 0 && end < SENTENCE_BUF_SIZE - 1 {
                        sentence_buf[0..end].copy_from_slice(&input_buffer[start..end + start]);
                        sentence_available = true;
                    }
                }
            }

            if sentence_available {
                Some(
                    core::str::from_utf8(&sentence_buf[0..end])
                        .unwrap()
                        .to_owned(),
                )
            } else {
                None
            }
        }
    }
}
