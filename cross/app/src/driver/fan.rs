use defmt::info;
use embassy_rp::{
    clocks,
    pwm::{self},
    Peripheral,
};
use embassy_time::Delay;
use embedded_hal_async::delay::DelayUs;
pub use fan_controller::decode::fan::Speed;
use fan_controller::{
    decode,
    units::{Frequency, Time},
};
use uom::si::{frequency::hertz, ratio::percent, time::millisecond};

type Result<T> = core::result::Result<T, Error>;

/// Represents a fan driver error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("not enough samples")]
    NotEnoughSamples(),
    /// A decode error occurred.
    #[error("decode error")]
    DecodeError(#[from] decode::fan::Error),
}

#[derive(derive_builder::Builder)]
#[builder(no_std, pattern = "owned")]
pub struct Fan<'a, C: pwm::Channel, T: pwm::Channel> {
    #[builder(setter(custom))]
    control: pwm::Pwm<'a, C>,
    #[builder(setter(custom))]
    tachometer: pwm::Pwm<'a, T>,
}

impl<'a, C: pwm::Channel, T: pwm::Channel> Fan<'a, C, T> {
    #[must_use]
    pub fn builder() -> FanBuilder<'a, C, T> {
        FanBuilder::default()
    }

    pub fn set_fan_speed(&mut self, speed: &Speed) {
        info!("setting fan to {}% speed", speed.get::<percent>());
        let params = speed.pwm_config(Frequency::new::<hertz>(f64::from(clocks::clk_sys_freq())));

        let mut config = pwm::Config::default();
        config.top = params.top;
        config.compare_a = params.compare;
        config.compare_b = params.compare;
        self.control.set_config(&config);
    }

    /// Returns the current fan rotation frequency.
    pub async fn fan_freq(&mut self) -> Result<Frequency> {
        let sample_duration = Time::new::<millisecond>(500.0);

        // We need to sample the number of pulses over a fixed duration to determine fan frequency.
        self.tachometer.set_counter(0);
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Delay {}
            .delay_ms(sample_duration.get::<millisecond>() as u32)
            .await;
        let pulse_count = self.tachometer.counter();

        if pulse_count < 2 {
            return Err(Error::NotEnoughSamples());
        }

        let frequency = 1.0 / (sample_duration / f64::from(pulse_count));
        Ok(frequency)
    }
}

impl<'a, C: pwm::Channel, T: pwm::Channel> FanBuilder<'a, C, T> {
    #[must_use]
    pub fn control(
        mut self,
        pin: impl Peripheral<P = impl pwm::PwmPinA<C>> + 'a,
        channel: impl Peripheral<P = C> + 'a,
    ) -> Self {
        self.control = Some(pwm::Pwm::new_output_a(channel, pin, pwm::Config::default()));
        self
    }

    #[must_use]
    pub fn tachometer(
        mut self,
        pin: impl Peripheral<P = impl pwm::PwmPinB<T>> + 'a,
        channel: impl Peripheral<P = T> + 'a,
    ) -> Self {
        self.tachometer = Some(pwm::Pwm::new_input(
            channel,
            pin,
            pwm::InputMode::FallingEdge,
            pwm::Config::default(),
        ));
        self
    }
}
