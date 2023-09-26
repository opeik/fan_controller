use embassy_rp::{gpio, pwm};
use fan_controller::{
    decode,
    fan_curve::{self, FanCurve},
};

use crate::driver::{self, fan::Fan};

type Result<T> = core::result::Result<T, Error>;

/// Represents a sensor error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    /// A fan decode error occurred.
    #[error("fan decode error: {0}")]
    FanDecodeError(#[from] decode::fan::Error),
    /// A fan curve error occurred.
    #[error("fan curve error: {0}")]
    FanCurveError(#[from] fan_curve::Error),
}

pub struct FanControl<'a, ControlPin, TachometerPin>
where
    ControlPin: pwm::Channel,
    TachometerPin: pwm::Channel,
{
    fan: Fan<'a, ControlPin, TachometerPin>,
    curve: FanCurve,
}

impl<'a, ControlPin, TachometerPin> FanControl<'a, ControlPin, TachometerPin>
where
    ControlPin: pwm::Channel,
    TachometerPin: pwm::Channel,
{
    pub fn new(fan: Fan<'a, ControlPin, TachometerPin>) -> Result<Self> {
        Ok(Self {
            fan,
            curve: FanCurve::new()?,
        })
    }

    pub async fn update(&mut self) -> Result<()> {
        // let sensor_data = self
        //     .temp_sensor
        //     .read()
        //     .await
        //     .map_err(Error::TemperatureSensorError)?;

        // let fan_power = self
        //     .curve
        //     .sample(sensor_data.temperature)
        //     .map_err(Error::FanCurveError)?;

        // self.fan.set_fan_power(&fan_power);
        Ok(())
    }
}
