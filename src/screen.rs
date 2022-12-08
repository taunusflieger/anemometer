pub mod anemometer_screen {
    use core::fmt::Debug;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{rectangle::Rectangle, PrimitiveStyleBuilder};
    use embedded_graphics::{mono_font::MonoFont, mono_font::MonoTextStyle, text::Text};
    use embedded_graphics_core::geometry::Point;
    use esp_idf_sys::EspError;
    use gfx_xtra::draw_target::Flushable;
    /*
        0                                                       239
    0   +-------------------------------------------------------+
        | Titel  18pt                             170           |
    22  +-----------------------------------------+-------------+
        |  GPS speed 24pt                         | m/s 18pt    |
    49  +-----------------------------------------+-------------+
        |  Wind speed  24pt            120        | m/s 18pt    |
    76  +------------------------------+----------+-------------+
        |  GPS connection state 14pt   | SD write activity 14pt |
        |                              |    180                 |
    94  +------------------------------+----+-------------------+
        |  IP address  10pt                 | SW Version 10pt   |
    129 +-----------------------------------+-------------------+

    gps_unit 170,49 239,22
    wind_speed 0,76 170,49
    wind_speed_unit 170,76 239,49
    gps_conn 0,94 120,76
    sd_write 120,94 239,76
    ip_address 0,129 180,94
    sw_version 180,129 239,94

    */
    #[derive(Copy, Clone, Debug)]
    pub struct BoundingBox {
        p1: Point, // lower left
        p2: Point, // upper right
    }

    pub struct TextWidget<'d> {
        bbox: BoundingBox,
        color: Rgb565,
        font: &'d MonoFont<'d>,
    }

    pub struct LayoutManager<'d> {
        pub title: TextWidget<'d>,
        pub gps_speed: TextWidget<'d>,
        pub gps_speed_unit: TextWidget<'d>,
        pub wind_speed: TextWidget<'d>,
        pub wind_speed_unit: TextWidget<'d>,
        pub gps_conn_state: TextWidget<'d>,
        pub sd_write: TextWidget<'d>,
        pub ip_address: TextWidget<'d>,
        pub sw_version: TextWidget<'d>,
    }

    impl<'d> LayoutManager<'d> {
        pub fn new() -> Result<LayoutManager<'d>, EspError> {
            let info_screen: LayoutManager<'d> = LayoutManager {
                title: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(0, 18),
                        p2: Point::new(239, 0),
                    },
                    color: Rgb565::RED,
                    font: &profont::PROFONT_18_POINT,
                },
                gps_speed: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(0, 49),
                        p2: Point::new(169, 18),
                    },
                    color: Rgb565::YELLOW,
                    font: &profont::PROFONT_24_POINT,
                },
                gps_speed_unit: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(171, 44),
                        p2: Point::new(239, 19),
                    },
                    color: Rgb565::YELLOW,
                    font: &profont::PROFONT_18_POINT,
                },
                wind_speed: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(0, 76),
                        p2: Point::new(169, 49),
                    },
                    color: Rgb565::GREEN,
                    font: &profont::PROFONT_24_POINT,
                },
                wind_speed_unit: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(171, 72),
                        p2: Point::new(239, 49),
                    },
                    color: Rgb565::GREEN,
                    font: &profont::PROFONT_18_POINT,
                },
                gps_conn_state: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(0, 94),
                        p2: Point::new(120, 76),
                    },
                    color: Rgb565::MAGENTA,
                    font: &profont::PROFONT_14_POINT,
                },
                sd_write: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(120, 94),
                        p2: Point::new(239, 76),
                    },
                    color: Rgb565::MAGENTA,
                    font: &profont::PROFONT_14_POINT,
                },
                ip_address: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(0, 129),
                        p2: Point::new(159, 94),
                    },
                    color: Rgb565::WHITE,
                    font: &profont::PROFONT_10_POINT,
                },
                sw_version: TextWidget {
                    bbox: BoundingBox {
                        p1: Point::new(160, 129),
                        p2: Point::new(239, 94),
                    },
                    color: Rgb565::WHITE,
                    font: &profont::PROFONT_10_POINT,
                },
            };
            Ok(info_screen)
        }

        pub fn draw_title<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.title, text)?;
            Ok(())
        }

        pub fn draw_gps_speed<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.gps_speed, text)?;
            Ok(())
        }

        pub fn draw_gps_speed_unit<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.gps_speed_unit, text)?;
            Ok(())
        }

        pub fn draw_wind_speed<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.wind_speed, text)?;
            Ok(())
        }

        pub fn draw_wind_speed_unit<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.wind_speed_unit, text)?;
            Ok(())
        }

        pub fn draw_gps_conn_state<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.gps_conn_state, text)?;
            Ok(())
        }

        pub fn draw_sd_write<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.sd_write, text)?;
            Ok(())
        }

        pub fn draw_ip_address<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.ip_address, text)?;
            Ok(())
        }

        pub fn draw_sw_version<D>(&self, target: &mut D, text: &str) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_widget(target, &self.sw_version, text)?;
            Ok(())
        }

        pub fn draw_widget<D>(
            &self,
            target: &mut D,
            widget: &TextWidget,
            text: &str,
        ) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            // erase background
            let style = PrimitiveStyleBuilder::new()
                //    .stroke_color(Rgb565::BLUE)
                //    .stroke_width(1)
                .fill_color(Rgb565::BLACK)
                .build();

            Rectangle::new(
                Point::new(widget.bbox.p1.x, widget.bbox.p2.y),
                Size::new(
                    (widget.bbox.p2.x - widget.bbox.p1.x) as u32,
                    (widget.bbox.p1.y - widget.bbox.p2.y) as u32,
                ),
            )
            .into_styled(style)
            .draw(target)?;

            let text_style = MonoTextStyle::new(widget.font, widget.color);
            Text::new(text, widget.bbox.p1, text_style).draw(target)?;

            Ok(())
        }

        pub fn draw_initial_screen<D>(&self, target: &mut D) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_title(target, "ESP32-S3 Anemometer")?;
            self.draw_gps_speed_unit(target, "km/h")?;
            self.draw_wind_speed_unit(target, "km/h")?;
            self.draw_gps_conn_state(target, "GPS: conn")?;
            self.draw_sd_write(target, "SD: w/r")?;

            Ok(())
        }

        #[allow(dead_code)]
        pub fn draw_demo_screen<D>(&self, target: &mut D) -> Result<(), D::Error>
        where
            D: Flushable<Color = Rgb565>,
            D::Error: Debug,
        {
            self.draw_title(target, "ESP32-S3 Anemometer")?;
            self.draw_gps_speed(target, "GPS: 30")?;
            self.draw_gps_speed_unit(target, "m/s")?;
            self.draw_wind_speed(target, "Win: 30")?;
            self.draw_wind_speed_unit(target, "m/s")?;
            self.draw_gps_conn_state(target, "GPS: conn")?;
            self.draw_sd_write(target, "SD: w/r")?;
            self.draw_ip_address(target, "192.168.200.1")?;
            self.draw_sw_version(target, "V0.1")?;

            Ok(())
        }
    }
}
