use defmt::info;
use embassy_rp::{
    clocks,
    pwm::{self},
};
use embassy_time::Delay;
use embedded_hal_async::delay::DelayUs;
pub use fan_controller::decode::fan::Power;
use fan_controller::{
    decode,
    units::{Frequency, Time},
};
use uom::si::{frequency::hertz, ratio::percent, time::millisecond};

type Result<T> = core::result::Result<T, Error>;

/// Represents a fan error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("not enough samples")]
    NotEnoughSamples(),
    /// A decode error occurred.
    #[error("pain")]
    DecodeError(#[from] decode::fan::Error),
}

pub struct Fan<'a, ControlPin, TachometerPin>
where
    ControlPin: pwm::Channel,
    TachometerPin: pwm::Channel,
{
    control_pin: pwm::Pwm<'a, ControlPin>,
    tachometer_pin: pwm::Pwm<'a, TachometerPin>,
}

impl<'a, ControlPin, TachometerPin> Fan<'a, ControlPin, TachometerPin>
where
    ControlPin: pwm::Channel,
    TachometerPin: pwm::Channel,
{
    pub fn new(
        control_pin: pwm::Pwm<'a, ControlPin>,
        tachometer_pin: pwm::Pwm<'a, TachometerPin>,
    ) -> Self {
        Self {
            control_pin,
            tachometer_pin,
        }
    }

    pub fn set_fan_power(&mut self, power: &Power) {
        info!("setting fan to {}% power", power.get::<percent>());
        let params = power.pwm_config(Frequency::new::<hertz>(f64::from(clocks::clk_sys_freq())));

        let mut config = pwm::Config::default();
        config.top = params.top;
        config.compare_a = params.compare;
        config.compare_b = params.compare;
        self.control_pin.set_config(&config);
    }

    /// Returns the current fan rotation frequency.
    pub async fn fan_freq(&mut self) -> Result<Frequency> {
        let sample_duration = Time::new::<millisecond>(500.0);

        // We need to sample the number of pulses over a fixed duration to determine fan frequency.
        self.tachometer_pin.set_counter(0);
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Delay {}
            .delay_ms(sample_duration.get::<millisecond>() as u32)
            .await;
        let pulse_count = self.tachometer_pin.counter();

        if pulse_count < 2 {
            return Err(Error::NotEnoughSamples());
        }

        let frequency = 1.0 / (sample_duration / f64::from(pulse_count));
        Ok(frequency)
    }
}
