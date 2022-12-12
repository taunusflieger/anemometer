pub mod sd_storage {

    use crate::peripherals::MicroSDCardPeripherals;
    use embedded_sdmmc::*;
    use esp_idf_hal::gpio::*;
    use esp_idf_hal::prelude::*;
    use esp_idf_hal::spi::config::Duplex;
    use esp_idf_hal::spi::*;
    use esp_idf_sys::*;
    use log::info;
    use std::rc::Rc;

    const FILE_TO_CREATE: &'static str = "GpsLog.txt";

    pub struct SdCard {
        pub sdmmc_spi: SdMmcSpi<
            SpiDeviceDriver<'static, Rc<SpiDriver<'static>>>,
            PinDriver<'static, AnyOutputPin, Output>,
        >,
    }

    impl SdCard {
        pub fn new(
            sdcard_peripherals: MicroSDCardPeripherals,
            driver: std::rc::Rc<SpiDriver<'static>>,
        ) -> Result<SdCard, EspError> {
            Ok(SdCard {
                sdmmc_spi: {
                    let sdmmc_spi = SpiDeviceDriver::new(
                        Rc::clone(&driver),
                        Option::<Gpio21>::None, // CS will be seperatly managed
                        &SpiConfig::new()
                            .duplex(Duplex::Full)
                            .baudrate(24.MHz().into()),
                    )?;

                    embedded_sdmmc::SdMmcSpi::new(
                        sdmmc_spi,
                        PinDriver::output(sdcard_peripherals.cs)?,
                    )
                },
            })
        }

        pub fn write(&mut self, data: String) {
            match self.sdmmc_spi.acquire() {
                Ok(block) => {
                    let mut controller: Controller<
                        BlockSpi<
                            '_,
                            esp_idf_hal::spi::SpiDeviceDriver<
                                '_,
                                Rc<esp_idf_hal::spi::SpiDriver<'_>>,
                            >,
                            esp_idf_hal::gpio::PinDriver<
                                '_,
                                esp_idf_hal::gpio::AnyOutputPin,
                                esp_idf_hal::gpio::Output,
                            >,
                        >,
                        SdMmcClock,
                        5,
                        5,
                    > = embedded_sdmmc::Controller::new(block, SdMmcClock);
                    info!("OK!");
                    info!("Card size...");
                    match controller.device().card_size_bytes() {
                        Ok(size) => info!("{}", size),
                        Err(e) => info!("Err: {:?}", e),
                    }
                    info!("Volume 0...");

                    let mut volume = match controller.get_volume(embedded_sdmmc::VolumeIdx(0)) {
                        Ok(v) => v,
                        Err(e) => panic!("Err: {:?}", e),
                    };

                    let root_dir = match controller.open_root_dir(&volume) {
                        Ok(d) => d,
                        Err(e) => panic!("Err: {:?}", e),
                    };

                    info!("creating file {}", FILE_TO_CREATE);
                    let mut f = match controller.open_file_in_dir(
                        &mut volume,
                        &root_dir,
                        FILE_TO_CREATE,
                        Mode::ReadWriteCreateOrAppend,
                    ) {
                        Ok(f) => f,
                        Err(e) => panic!("Err: {:?}", e),
                    };

                    f.seek_from_end(0).unwrap();
                    let num_written = match controller.write(&mut volume, &mut f, data.as_bytes()) {
                        Ok(num) => num,
                        Err(e) => panic!("Err: {:?}", e),
                    };
                    info!("Bytes written {}", num_written);
                    match controller.close_file(&volume, f) {
                        Ok(_) => info!("file closed"),
                        Err(e) => panic!("Err: {:?}", e),
                    };
                }
                Err(e) => info!("Error acquire SPI bus {:?}", e),
            };
        }
    }
    pub struct SdMmcClock;

    impl TimeSource for SdMmcClock {
        fn get_timestamp(&self) -> Timestamp {
            Timestamp {
                year_since_1970: 0,
                zero_indexed_month: 0,
                zero_indexed_day: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            }
        }
    }
}
