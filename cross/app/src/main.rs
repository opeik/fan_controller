#![no_std]
#![no_main]
#![feature(async_fn_in_trait, error_in_core, type_alias_impl_trait)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::used_underscore_binding, clippy::missing_errors_doc)]

pub mod driver;
pub mod fan_control;

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    config::{self},
    gpio::{self, Level, Output},
    i2c, peripherals,
    pwm::{self},
};
use embassy_time::{Duration, Timer};
use fan_controller::decode::mcp9808;
use panic_probe as _;

bind_interrupts!(struct Interrupts {
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(config::Config::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    // let mut i2c = i2c::I2c::new_async(
    //     peripherals.I2C0,
    //     peripherals.PIN_17,
    //     peripherals.PIN_16,
    //     Interrupts,
    //     i2c::Config::default(),
    // );

    // i2c.write_async(
    //     mcp9808::DEFAULT_ADDRESS,
    //     [mcp9808::Register::Temperature as u8],
    // )
    // .await
    // .unwrap();

    // let mut fan = Fan::new(
    //     pwm::Pwm::new_output_b(
    //         peripherals.PWM_CH7,
    //         peripherals.PIN_15,
    //         pwm::Config::default(),
    //     ),
    //     pwm::Pwm::new_input(
    //         peripherals.PWM_CH0,
    //         peripherals.PIN_17,
    //         pwm::InputMode::FallingEdge,
    //         pwm::Config::default(),
    //     ),
    // );

    // let mut temp_sensor = Dht11::new(gpio::OutputOpenDrain::new(peripherals.PIN_16, Level::High));
    // fan.set_fan_power(&Power::new(Ratio::new::<percent>(100.0)).unwrap());

    loop {
        led.toggle();
        // match fan.fan_freq().await {
        //     Ok(v) => info!(
        //         "fan speed: {}Hz, {}RPM",
        //         v.get::<hertz>(),
        //         (v.get::<hertz>() * 60.0) / 2.0
        //     ),
        //     Err(e) => error!("failed to read fan speed: {}", e),
        // }

        // let mut payload: [u8; 2] = [0; 2];
        // i2c.read_async(mcp9808::DEFAULT_ADDRESS, &mut payload)
        //     .await
        //     .unwrap();

        Timer::after(Duration::from_secs(1)).await;
    }
}
