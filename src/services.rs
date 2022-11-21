use crate::errors::*;
use crate::peripherals::DisplaySpiPeripherals;
use core::fmt::Debug;
use core::mem;

#[cfg(feature = "tft")]
use display_interface_spi::SPIInterfaceNoCS;

#[cfg(feature = "tft")]
use embedded_graphics::{pixelcolor::Rgb565, prelude::*, primitives::*};
use esp_idf_hal::delay;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::spi::*;

#[cfg(feature = "tft")]
use mipidsi::{models::ST7789, Builder, Display};

#[cfg(feature = "tft")]
fn display(
    display_peripherals: DisplaySpiPeripherals<
        impl Peripheral<P = impl SpiAnyPins + 'static> + 'static,
    >,
) -> Display<
    SPIInterfaceNoCS<
        SpiExclusiveDevice<QSPI1, (Pin3<IOF0<NoInvert>>, (), Pin5<IOF0<NoInvert>>)>,
        Pin13<Output<Regular<NoInvert>>>,
    >,
    Pin11<Output<Regular<NoInvert>>>,
    ST7789,
> {
    if let Some(backlight) = display_peripherals.control.backlight {
        let mut backlight = PinDriver::output(backlight)?;

        backlight.set_drive_strength(DriveStrength::I40mA)?;
        backlight.set_high()?;

        mem::forget(backlight); // TODO: For now
    }

    let baudrate = 26.MHz().into();
    //let baudrate = 40.MHz().into();

    let spi_display = SpiDeviceDriver::new_single(
        display_peripherals.spi,
        display_peripherals.sclk,
        display_peripherals.sdo,
        Option::<Gpio21>::None,
        Dma::Disabled,
        display_peripherals.cs,
        &SpiConfig::new().baudrate(baudrate),
    )?;

    let dc = PinDriver::output(display_peripherals.control.dc)?;

    let di = SPIInterfaceNoCS::new(spi_display, dc);

    let mut display = Builder::st7789(di) // known model or with_model(model)
        .with_display_size(240, 135) // set any options on the builder before init
        .init(&mut delay::Ets, Some(rst_pin)); // optional reset pin

    let circle1 =
        Circle::new(Point::new(128, 64), 64).into_styled(PrimitiveStyle::with_fill(Rgb565::RED));
    let circle2 = Circle::new(Point::new(64, 64), 64)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));

    let blue_with_red_outline = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLUE)
        .stroke_color(Rgb565::RED)
        .stroke_width(1) // > 1 is not currently supported in embedded-graphics on triangles
        .build();
    let triangle = Triangle::new(
        Point::new(40, 120),
        Point::new(40, 220),
        Point::new(140, 120),
    )
    .into_styled(blue_with_red_outline);

    let line = Line::new(Point::new(180, 160), Point::new(239, 239))
        .into_styled(PrimitiveStyle::with_stroke(RgbColor::WHITE, 10));

    // draw two circles on black background

    display.clear(Rgb565::BLACK).unwrap();
    circle1.draw(&mut display).unwrap();
    circle2.draw(&mut display).unwrap();
    triangle.draw(&mut display).unwrap();
    line.draw(&mut display).unwrap();
}

/*
pub fn display(
    peripherals: DisplaySpiPeripherals<impl Peripheral<P = impl SpiAnyPins + 'static> + 'static>,
) -> Result<
    Display<
        SPIInterfaceNoCS<SpiDeviceDriver<SpiDriver>, PinDriver<AnyOutputPin, Output>>,
        ST7789,
        AnyOutputPin,
    >,
    InitError<<AnyOutputPin as OutputPin>::Error>,
> {
    //Result<impl Flushable<Color = Color, Error = impl Debug + 'static> + 'static, InitError> {
    if let Some(backlight) = peripherals.control.backlight {
        let mut backlight = PinDriver::output(backlight)?;

        backlight.set_drive_strength(DriveStrength::I40mA)?;
        backlight.set_high()?;

        mem::forget(backlight); // TODO: For now
    }

    let baudrate = 26.MHz().into();
    //let baudrate = 40.MHz().into();

    let spi_display = SpiDeviceDriver::new_single(
        peripherals.spi,
        peripherals.sclk,
        peripherals.sdo,
        Option::<Gpio21>::None,
        Dma::Disabled,
        peripherals.cs,
        &SpiConfig::new().baudrate(baudrate),
    )?;

    let dc = PinDriver::output(peripherals.control.dc)?;

    let di = SPIInterfaceNoCS::new(spi_display, dc);

    let display = Builder::st7789(di) // known model or with_model(model)
        .with_display_size(240, 135) // set any options on the builder before init
        .init(&mut delay::Ets, Some(peripherals.control.rst)); // optional reset pin

    //let display = display.owned_color_converted().owned_noop_flushing();

    Ok(display)
}
*/
