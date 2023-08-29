#![cfg_attr(not(test), no_std)]
#![feature(error_in_core)]

use onlyerror::Error;

/// Fan speed, represented as a percentage ranging `0..=100`.
pub struct FanSpeed(f64);

#[derive(Debug, Error)]
pub enum FanError {
    #[error("invalid fan speed `{0}`, expected 0..=1")]
    InvalidFanSpeed(f64),
}

impl FanSpeed {
    /// Creates a new `FanSpeed`.
    pub fn new(fan_speed: f64) -> core::result::Result<FanSpeed, FanError> {
        if !(0.0..=1.0).contains(&fan_speed) {
            return Err(FanError::InvalidFanSpeed(fan_speed));
        }

        Ok(FanSpeed(fan_speed))
    }
}

// impl From<FanSpeed> for PwmConfig {
//     fn from(value: FanSpeed) -> Self {
//         const FAN_PWM_HZ: f64 = 25_000.0;
//         let clock_hz = embassy_rp::clocks::clk_sys_freq() as f64;

//         let mut pwm_config = PwmConfig::default();
//         pwm_config.top = (clock_hz / FAN_PWM_HZ) as u16;
//         pwm_config.compare_a = ((pwm_config.top as f64) * value.0) as u16;
//         pwm_config.compare_b = pwm_config.compare_a;

//         pwm_config
//     }
// }

mod tests {
    #[test]
    fn test_add() {
        assert_eq!(1, 1);
    }
}
