use uom::si::{
    f32::{Frequency, Ratio},
    frequency::hertz,
};

/// Represents desired fan speed.
pub struct FanSpeed(pub Ratio);

/// Represents RP2040 PWM parameters.
#[derive(Debug, PartialEq)]
pub struct PwmParams {
    pub top: u16,
    pub compare: u16,
}

impl FanSpeed {
    pub fn to_pwm_params(&self, clock: Frequency) -> PwmParams {
        // As specified by Intel "4-Wire Pulse Width Modulation (PWM) Controlled Fans".
        let fan_pwm_signal = Frequency::new::<hertz>(25_000.0);

        let fan_speed = self.0;
        let top = clock / fan_pwm_signal;
        let compare = top * fan_speed;
        PwmParams {
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
            FanSpeed(Ratio::new::<percent>(0.0))
                .to_pwm_params(Frequency::new::<hertz>(125_000_000.0)),
            PwmParams {
                top: 5000,
                compare: 0,
            }
        );

        assert_eq!(
            FanSpeed(Ratio::new::<percent>(50.0))
                .to_pwm_params(Frequency::new::<hertz>(125_000_000.0)),
            PwmParams {
                top: 5000,
                compare: 2500,
            }
        );

        assert_eq!(
            FanSpeed(Ratio::new::<percent>(100.0))
                .to_pwm_params(Frequency::new::<hertz>(125_000_000.0)),
            PwmParams {
                top: 5000,
                compare: 5000,
            }
        );
    }
}
