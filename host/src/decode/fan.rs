use core::time::Duration;

use heapless::Vec;
use uom::si::{self, f64::Frequency, frequency::hertz, ratio::percent};

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a fan error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error {
    /// The checksum is mismatched.
    #[error("invalid fan power: expected 0≤x≤100%, got {0}%")]
    InvalidPower(f64),
    #[error("not enough samples: expected x≥2, got {0} samples")]
    NotEnoughSamples(usize),
}

/// Represents desired fan power.
#[derive(derive_more::Deref)]
pub struct Power(si::f64::Ratio);

/// Represents RP2040 PWM parameters.
#[derive(Debug, PartialEq, defmt::Format)]
pub struct RpPwmConfig {
    pub top: u16,
    pub compare: u16,
}

impl Power {
    const NUM_SAMPLES: usize = 30;

    pub fn new(ratio: si::f64::Ratio) -> Result<Self> {
        let inner = ratio.get::<percent>();

        if (0.0..=100.0).contains(&inner) {
            Ok(Self(ratio))
        } else {
            Err(Error::InvalidPower(inner))
        }
    }

    #[must_use]
    pub fn pwm_config(&self, clock: si::f64::Frequency) -> RpPwmConfig {
        // As specified by Intel "4-Wire Pulse Width Modulation (PWM) Controlled Fans".
        let fan_pwm_signal = si::f64::Frequency::new::<hertz>(25_000.0);
        let fan_speed = self.0;
        let top = clock / fan_pwm_signal;
        let compare = top * fan_speed;

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        RpPwmConfig {
            top: top.value as u16,
            compare: compare.value as u16,
        }
    }

    pub fn fan_freq<'a, I: IntoIterator<Item = &'a Duration>>(
        samples: I,
    ) -> Result<si::f64::Frequency> {
        const SAMPLES_PER_ROTATION: f64 = 2.0;
        let samples = samples
            .into_iter()
            .collect::<Vec<_, { Self::MAX_SAMPLES }>>();
        let count = samples.iter().count();

        if count < 2 {
            return Err(Error::NotEnoughSamples(count));
        }

        let sum = samples.iter().copied().sum::<Duration>().as_secs_f64();
        #[allow(clippy::cast_precision_loss)]
        let avg = sum / count as f64;
        let frequency = (1.0 / avg) / SAMPLES_PER_ROTATION;
        Ok(Frequency::new::<hertz>(frequency))
    }
}

#[cfg(test)]
mod tests {
    use uom::si::{
        f64::{Frequency, Ratio},
        ratio::percent,
    };

    use super::*;

    #[test]
    fn to_pwm_params() {
        assert_eq!(
            Power(Ratio::new::<percent>(0.0)).pwm_config(Frequency::new::<hertz>(125_000_000.0)),
            RpPwmConfig {
                top: 5000,
                compare: 0,
            }
        );

        assert_eq!(
            Power(Ratio::new::<percent>(50.0)).pwm_config(Frequency::new::<hertz>(125_000_000.0)),
            RpPwmConfig {
                top: 5000,
                compare: 2500,
            }
        );

        assert_eq!(
            Power(Ratio::new::<percent>(100.0)).pwm_config(Frequency::new::<hertz>(125_000_000.0)),
            RpPwmConfig {
                top: 5000,
                compare: 5000,
            }
        );
    }
}
