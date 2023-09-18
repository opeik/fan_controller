#![no_std]
#![no_main]
#![feature(async_fn_in_trait)]
#![feature(error_in_core)]
#![feature(type_alias_impl_trait)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::used_underscore_binding)]

mod driver;
mod future;

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    config::{self},
    gpio::{self, Level, Output},
    pwm::{self},
};
use embassy_time::{Duration, Timer};
use fan_controller::decode::fan::Power;
use panic_probe as _;
use uom::si::{f64::Ratio, frequency::hertz, ratio::percent};

use crate::driver::{dht::Dht11, fan::Fan};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(config::Config::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    let mut fan = Fan::new(
        pwm::Pwm::new_output_a(
            peripherals.PWM_CH1,
            peripherals.PIN_2,
            pwm::Config::default(),
        ),
        pwm::Pwm::new_input(
            peripherals.PWM_CH0,
            peripherals.PIN_17,
            pwm::InputMode::FallingEdge,
            pwm::Config::default(),
        ),
    );

    let mut _temp_sensor = Dht11::new(gpio::OutputOpenDrain::new(peripherals.PIN_16, Level::High));
    info!("waiting for DHT11 to initialize...");
    Timer::after(Duration::from_secs(1)).await;
    fan.set_fan_power(&Power::new(Ratio::new::<percent>(50.0)).unwrap());

    loop {
        led.toggle();
        match fan.fan_speed().await {
            Ok(v) => info!(
                "fan speed: {}Hz, {}RPM",
                v.get::<hertz>(),
                (v.get::<hertz>() * 60.0) / 2.0
            ),
            Err(e) => error!("failed to read fan speed: {}", e),
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}
