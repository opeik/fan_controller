#![no_std]
#![no_main]
#![feature(async_fn_in_trait)]
#![feature(error_in_core)]
#![feature(type_alias_impl_trait)]

mod driver;
mod future;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{self, Level, Output, OutputOpenDrain},
    pwm::{self},
};
use embassy_time::{Delay, Duration, Timer};
use fan_controller::fan::Speed;
use panic_probe as _;
use uom::si::{
    f64::{Frequency, Ratio},
    frequency::hertz,
    ratio::percent,
};

use crate::driver::{dht::Dht11, fan::Fan};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    let mut fan = Fan::new(
        pwm::Pwm::new_output_b(
            peripherals.PWM_CH7,
            peripherals.PIN_15,
            pwm::Config::default(),
        ),
        gpio::Input::new(peripherals.PIN_17, gpio::Pull::None),
    );

    fan.set_fan_speed(Speed());

    let mut temp_sensor = Dht11::new(OutputOpenDrain::new(peripherals.PIN_16, Level::High));
    info!("waiting for DHT11 to initialize...");
    Timer::after(Duration::from_secs(1)).await;

    loop {
        led.toggle();

        match temp_sensor.read().await {
            Ok(v) => info!("{:?}", v),
            Err(e) => info!("failed to read sensor: {}", e),
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}
