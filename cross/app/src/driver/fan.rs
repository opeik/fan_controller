use embassy_rp::{
    clocks,
    gpio::{self},
    pwm::{self},
};
use fan_controller::fan::Speed;
use uom::si::{f64::Frequency, frequency::hertz};

pub struct Fan<'a, PwmChannel, GpioPin>
where
    PwmChannel: pwm::Channel,
    GpioPin: gpio::Pin,
{
    control_pin: pwm::Pwm<'a, PwmChannel>,
    tachometer_pin: gpio::Input<'a, GpioPin>,
}

impl<'a, PwmChannel, GpioPin> Fan<'a, PwmChannel, GpioPin>
where
    PwmChannel: pwm::Channel,
    GpioPin: gpio::Pin,
{
    pub fn new(
        control_pin: pwm::Pwm<'a, PwmChannel>,
        tachometer_pin: gpio::Input<'a, GpioPin>,
    ) -> Self {
        Self {
            control_pin,
            tachometer_pin,
        }
    }

    pub fn set_fan_speed(&mut self, fan_speed: Speed) {
        let params =
            fan_speed.to_pwm_params(Frequency::new::<hertz>(clocks::clk_sys_freq() as f64));

        let mut config = pwm::Config::default();
        config.top = params.top;
        config.compare_a = params.compare;
        config.compare_b = params.compare;

        self.control_pin.set_config(&config)
    }
}
