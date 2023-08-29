#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(error_in_core)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{self, Level, Output},
    pwm::{Config as PwmConfig, Pwm},
};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    // let pwm_config = PwmConfig::from(FanSpeed::new(1.0).unwrap());

    // let _pwm = Pwm::new_output_a(peripherals.PWM_CH2, peripherals.PIN_20, pwm_config);
    // info!("pwm initialized!");

    let mut is_led_on = false;
    loop {
        is_led_on = !is_led_on;
        match is_led_on {
            true => led.set_high(),
            false => led.set_low(),
        }

        Timer::after(Duration::from_millis(500)).await;
    }
}
