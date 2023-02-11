#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_time::{Duration, Timer};
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

const OFF_PULSE_RATIO: u64 = 12;
const ON_PULSE_RATIO: u64 = 1;
const BASE_PULSE_WIDTH: u64 = 20;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut led = Output::new(p.PIN_25, Level::Low);
    let mut signal = Output::new(p.PIN_16, Level::Low);

    loop {
        //info!("led on!");
        //led.set_high();
        signal.set_high();
        Timer::after(Duration::from_millis(ON_PULSE_RATIO * BASE_PULSE_WIDTH)).await;

        //info!("led off!");
        //led.set_low();
        signal.set_low();
        Timer::after(Duration::from_millis(OFF_PULSE_RATIO * BASE_PULSE_WIDTH)).await;
    }
}
