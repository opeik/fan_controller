#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(error_in_core)]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output, OutputOpenDrain},
    pwm::{Channel, Config as PwmConfig, Pwm},
};
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_async::digital::Wait;
use fan_controller::{dht::Dht11, fan::FanSpeed};
use panic_probe as _;
use uom::si::{
    f32::{Frequency, Ratio},
    frequency::hertz,
    ratio::percent,
};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());
    let mut led = Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    let mut pwm = Pwm::new_output_b(
        peripherals.PWM_CH7,
        peripherals.PIN_15,
        PwmConfig::default(),
    );
    set_fan_speed(&mut pwm, FanSpeed(Ratio::new::<percent>(0.0)));
    info!("pwm initialized!");

    let dht_pin = OutputOpenDrain::new(peripherals.PIN_16, Level::High);
    let dht = Dht11::new(dht_pin, Delay);

    let mut is_led_on = false;
    loop {
        is_led_on = !is_led_on;
        match is_led_on {
            true => led.set_high(),
            false => led.set_low(),
        }

        Timer::after(Duration::from_secs(1)).await;
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
