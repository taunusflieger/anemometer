use crate::errors::*;
use crate::peripherals::DisplaySpiPeripherals;
use core::fmt::Debug;
use core::mem;
use esp_idf_hal::prelude::*;

#[cfg(feature = "tft")]
use display_interface_spi::SPIInterfaceNoCS;

#[cfg(feature = "tft")]
use embedded_graphics::pixelcolor::Rgb565;

use esp_idf_hal::delay;

use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::spi::*;
use gfx_xtra::draw_target::{Flushable, OwnedDrawTargetExt};

#[cfg(feature = "tft")]
use mipidsi::{Builder, Orientation};

#[cfg(feature = "tft")]
pub fn display(
    display_peripherals: DisplaySpiPeripherals<
        impl Peripheral<P = impl SpiAnyPins + 'static> + 'static,
        impl Peripheral<P = impl OutputPin + 'static> + 'static,
    >,
) -> Result<impl Flushable<Color = Rgb565, Error = impl Debug + 'static> + 'static, InitError> {
    // power ST7789
    let mut vdd = PinDriver::output(display_peripherals.vdd)?;
    vdd.set_high()?;
    mem::forget(vdd);

    let baudrate = 80.MHz().into();

    let spi_display = SpiDeviceDriver::new_single(
        display_peripherals.spi,
        display_peripherals.sclk,
        display_peripherals.sdo,
        Option::<Gpio21>::None,
        Dma::Disabled,
        Some(display_peripherals.cs),
        &SpiConfig::new().baudrate(baudrate),
    )
    .unwrap();

    let dc = PinDriver::output(display_peripherals.control.dc).unwrap();
    let di = SPIInterfaceNoCS::new(spi_display, dc);
    let rst = PinDriver::output(display_peripherals.control.rst).unwrap();

    // create driver
    let display = Builder::st7789_pico1(di)
        .with_display_size(135, 240)
        // set default orientation
        .with_orientation(Orientation::Landscape(true))
        // initialize
        .init(&mut delay::Ets, Some(rst))
        .unwrap();

    let display = display.owned_noop_flushing();
    Ok(display)
}
