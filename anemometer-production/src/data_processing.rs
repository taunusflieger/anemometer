use heapless::HistoryBuffer;

// TODO: adjust factor once calibration is done
const RPM_TO_KMH: f32 = 1.0;

#[inline(always)]
fn rpm_to_kmh<T: Into<f32>>(rpm: T) -> f32 {
    rpm.into() * RPM_TO_KMH
}

// This strucure is updated every 500ms with measurement data.
// Mesurement resolution is 1 sec. The process of processing
// wind data follows the guidance provided in the following document;
//
// Guide to Meteorological Instruments and
// Methods of Observation WMO-No. 9
// 2014 edition
// Updated in 2017
// According to section 1.3.2.4 Instantaneous meteorological values
// "In order to standardize averaging algorithms it is recommended:
//
// (a) That atmospheric pressure, air temperature, air humidity, sea-surface temperature, visibility,
// among others, be reported as 1 to 10 min averages, which are obtained after linearization
// of the sensor output;
//
// (b) That wind, except wind gusts, be reported as 2 or 10 min averages, which are obtained after
// linearization of the sensor output.
//
// These averaged values are to be considered as the “instantaneous” values of meteorological
// variables"
//
// The data stored in the structure are raw data which are in case of wind speed
// rotations per second
#[allow(dead_code)]
pub struct WindDataHistory {
    // 2 min measuremnt interval with a 2 Hz sampling frequency
    wind_gust_buffer: HistoryBuffer<u16, 6>,
    // 2 min measuremnt interval with a 2 Hz sampling frequency
    wind_speed_buffer: HistoryBuffer<u16, 240>,
    // 2 min measuremnt interval with a 2 Hz sampling frequency
    wind_direction_buffer: HistoryBuffer<u16, 240>,
    // maximum wind guest within the 2min interval
    wind_gust: f32,
}

impl WindDataHistory {
    #[allow(dead_code)]
    pub fn new(&self) -> Self {
        WindDataHistory {
            wind_gust_buffer: HistoryBuffer::new(),
            wind_speed_buffer: HistoryBuffer::new(),
            wind_direction_buffer: HistoryBuffer::new(),
            wind_gust: 0.0,
        }
    }
    #[allow(dead_code)]
    pub fn store_measurement(&mut self, speed: u16, direction: u16) {
        self.wind_speed_buffer.write(speed);
        self.wind_gust_buffer.write(speed);
        self.wind_direction_buffer.write(direction);

        // calculate avg win guest and check if it higher than the previous
        // wind gust
        let avg = self.wind_gust_buffer.as_slice().iter().sum::<u16>() as f32
            / self.wind_gust_buffer.len() as f32;
        if avg > self.wind_gust {
            self.wind_gust = avg;
        }
    }
}

impl Default for WindDataHistory {
    fn default() -> Self {
        Self {
            wind_gust_buffer: HistoryBuffer::new(),
            wind_speed_buffer: HistoryBuffer::new(),
            wind_direction_buffer: HistoryBuffer::new(),
            wind_gust: 0.0,
        }
    }
}

pub trait WindStatistics {
    fn avg_speed(&self) -> f32;

    fn avg_direction(&self) -> f32;

    fn gust_speed(&self) -> f32;

    fn max_speed(&self) -> f32;

    fn clear_wind_gust(&mut self);
}

impl WindStatistics for WindDataHistory {
    fn avg_speed(&self) -> f32 {
        let avg = self.wind_speed_buffer.as_slice().iter().sum::<u16>() as f32
            / self.wind_speed_buffer.len() as f32;
        rpm_to_kmh(avg)
    }

    fn avg_direction(&self) -> f32 {
        todo!();
    }

    fn gust_speed(&self) -> f32 {
        rpm_to_kmh(self.wind_gust)
    }

    fn max_speed(&self) -> f32 {
        if let Some(max) = self.wind_speed_buffer.as_slice().iter().max() {
            rpm_to_kmh(*max)
        } else {
            0.0
        }
    }

    fn clear_wind_gust(&mut self) {
        self.wind_gust = 0.0;
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn avg_speed_test() {
        let mut wind_data = WindDataHistory::default();

        for _ in 0..240 {
            wind_data.store_measurement(1, 0);
        }

        assert_eq!(wind_data.avg_speed(), 1.0);
    }

    #[test]
    fn gust_speed_test() {
        let mut wind_data = WindDataHistory::default();

        for _ in 0..240 {
            wind_data.store_measurement(1, 0);
        }

        let gust = wind_data.gust_speed();
        assert_eq!(gust, 1.0);
    }
}
