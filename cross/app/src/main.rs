#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(error_in_core)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output},
    pwm::{Channel, Config as PwmConfig, Pwm},
};
use embassy_time::{Duration, Timer};
use fan_controller::FanSpeed;
use uom::si::{f32::*, frequency::hertz, ratio::percent};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    let mut pwm = Pwm::new_output_a(
        peripherals.PWM_CH2,
        peripherals.PIN_20,
        PwmConfig::default(),
    );
    info!("pwm initialized!");

    let mut is_led_on = false;
    let mut fan_speed = 0.0f32;
    loop {
        fan_speed = (fan_speed + 10.0) % 100.0;
        set_fan_speed(&mut pwm, FanSpeed::new(Ratio::new::<percent>(fan_speed)));

        is_led_on = !is_led_on;
        match is_led_on {
            true => led.set_high(),
            false => led.set_low(),
        }

        info!("fan speed: {}", fan_speed);
        Timer::after(Duration::from_millis(2000)).await;
    }
}

fn set_fan_speed<T: Channel>(pwm: &mut Pwm<T>, fan_speed: FanSpeed) {
    let params = fan_speed.to_pwm_params(Frequency::new::<hertz>(125_000_000.0));

    let mut config = PwmConfig::default();
    config.top = params.top;
    config.compare_a = params.compare;
    config.compare_b = params.compare;

    pwm.set_config(&config)
}
