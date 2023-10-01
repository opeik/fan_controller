use uom::si::{self, frequency::hertz, ratio::percent};

use crate::units::{Frequency, Ratio};

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
#[derive(Default, derive_more::Deref)]
pub struct Power(Ratio);

/// Represents RP2040 PWM parameters.
#[derive(Debug, Copy, Clone, PartialEq, defmt::Format)]
pub struct RpPwmConfig {
    pub top: u16,
    pub compare: u16,
}

impl Power {
    pub fn new(ratio: Ratio) -> Result<Self> {
        let inner = ratio.get::<percent>();

        if (0.0..=100.0).contains(&inner) {
            Ok(Self(ratio))
        } else {
            Err(Error::InvalidPower(inner))
        }
    }

    #[must_use]
    pub fn pwm_config(&self, clock: Frequency) -> RpPwmConfig {
        // As specified by Intel "4-Wire Pulse Width Modulation (PWM) Controlled Fans".
        let fan_pwm_signal = Frequency::new::<hertz>(25_000.0);
        let fan_speed = self.0;
        let top = clock / fan_pwm_signal;
        let compare = top * fan_speed;

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        RpPwmConfig {
            top: top.value as u16,
            compare: compare.value as u16,
        }
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
    fn fan_power_to_pwm_config() {
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
