use uom::si::{
    f64::{Frequency, Ratio},
    frequency::hertz,
    ratio::percent,
};

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a fan error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error {
    /// The checksum is mismatched.
    #[error("invalid fan speed: expected 0≤x≤100%, got {0}%")]
    InvalidSpeed(f64),
}

/// Represents desired fan speed.
pub struct Speed(Ratio);

/// Represents RP2040 PWM parameters.
#[derive(Debug, PartialEq)]
pub struct RpPwmParams {
    pub top: u16,
    pub compare: u16,
}

impl Speed {
    pub fn new(ratio: Ratio) -> Result<Self> {
        let inner = ratio.get::<percent>();

        match (0.0..=100.0).contains(&inner) {
            false => Err(Error::InvalidSpeed(inner)),
            true => Ok(Self(ratio)),
        }
    }

    pub fn to_pwm_params(&self, clock: Frequency) -> RpPwmParams {
        // As specified by Intel "4-Wire Pulse Width Modulation (PWM) Controlled Fans".
        let fan_pwm_signal = Frequency::new::<hertz>(25_000.0);
        let fan_speed = self.0;
        let top = clock / fan_pwm_signal;
        let compare = top * fan_speed;
        RpPwmParams {
            top: top.value as u16,
            compare: compare.value as u16,
        }
    }
}

#[cfg(test)]
mod tests {
    use uom::si::ratio::percent;

    use super::*;

    #[test]
    fn to_pwm_params() {
        assert_eq!(
            Speed(Ratio::new::<percent>(0.0)).to_pwm_params(Frequency::new::<hertz>(125_000_000.0)),
            RpPwmParams {
                top: 5000,
                compare: 0,
            }
        );

        assert_eq!(
            Speed(Ratio::new::<percent>(50.0))
                .to_pwm_params(Frequency::new::<hertz>(125_000_000.0)),
            RpPwmParams {
                top: 5000,
                compare: 2500,
            }
        );

        assert_eq!(
            Speed(Ratio::new::<percent>(100.0))
                .to_pwm_params(Frequency::new::<hertz>(125_000_000.0)),
            RpPwmParams {
                top: 5000,
                compare: 5000,
            }
        );
    }
}
