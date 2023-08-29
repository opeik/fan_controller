#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::clocks;
use embassy_rp::gpio;
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    let pwm_config = pwm_config_from_fan_speed(0.0);
    info!(
        "compare_a = {}, compare_b = {}, top = {}",
        pwm_config.compare_a, pwm_config.compare_b, pwm_config.top,
    );

    let _pwm = Pwm::new_output_a(peripherals.PWM_CH2, peripherals.PIN_20, pwm_config);
    info!("pwm initialized!");

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

fn pwm_config_from_fan_speed(fan_perc: f32) -> PwmConfig {
    const FAN_PWM_HZ: f32 = 25_000.0;
    let clock_hz = embassy_rp::clocks::clk_sys_freq() as f32;

    let mut pwm_config = PwmConfig::default();
    pwm_config.top = (clock_hz / FAN_PWM_HZ) as u16;
    pwm_config.compare_a = ((pwm_config.top as f32) * fan_perc) as u16;
    pwm_config.compare_b = pwm_config.compare_a;

    pwm_config
}
