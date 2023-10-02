#![no_std]
#![no_main]
#![feature(async_fn_in_trait, error_in_core, type_alias_impl_trait)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::used_underscore_binding, clippy::missing_errors_doc)]

pub mod driver;
pub mod fan_control;

use defmt::{error, info};
use defmt_rtt as _;
use driver::mcp9808::Mcp9808;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    config::{self},
    gpio::{self, Level},
    i2c, peripherals,
};
use embassy_time::{Duration, Timer};
use panic_probe as _;

use crate::{driver::fan::Fan, fan_control::FanControl};

bind_interrupts!(struct Interrupts {
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(config::Config::default());
    let mut led = gpio::Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    let mut fan_control = FanControl::new(
        Fan::new(
            peripherals.PWM_CH0,
            peripherals.PIN_0,
            peripherals.PWM_CH1,
            peripherals.PIN_3,
        ),
        Mcp9808::new(
            peripherals.I2C0,
            peripherals.PIN_17,
            peripherals.PIN_16,
            Interrupts,
        ),
    )
    .unwrap();

    loop {
        led.toggle();
        match fan_control.update().await {
            Ok(()) => info!("fan updated"),
            Err(e) => error!("error: {}", e),
        };
        Timer::after(Duration::from_secs(1)).await;
    }
}
