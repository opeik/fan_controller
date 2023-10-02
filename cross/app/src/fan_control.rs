use embassy_rp::{i2c, pwm};
use fan_controller::fan_curve::{self, FanCurve};

use crate::driver::{self, fan::Fan, mcp9808::Mcp9808};

type Result<T> = core::result::Result<T, Error>;

/// Represents a sensor error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    /// A fan driver error occurred.
    #[error("temperature sensor driver error: {0}")]
    TempSensorError(#[from] driver::mcp9808::Error),
    /// A fan decode error occurred.
    #[error("fan driver error: {0}")]
    FanDecodeError(#[from] driver::fan::Error),
    /// A fan curve error occurred.
    #[error("fan curve error: {0}")]
    FanCurveError(#[from] fan_curve::Error),
}

pub struct FanControl<'a, ControlChannel, TachometerChannel, SensorI2C>
where
    ControlChannel: pwm::Channel,
    TachometerChannel: pwm::Channel,
    SensorI2C: i2c::Instance,
{
    fan: Fan<'a, ControlChannel, TachometerChannel>,
    curve: FanCurve,
    sensor: Mcp9808<'a, SensorI2C>,
}

impl<'a, ControlChannel, TachometerChannel, SensorI2C>
    FanControl<'a, ControlChannel, TachometerChannel, SensorI2C>
where
    ControlChannel: pwm::Channel,
    TachometerChannel: pwm::Channel,
    SensorI2C: i2c::Instance,
{
    pub fn new(
        fan: Fan<'a, ControlChannel, TachometerChannel>,
        sensor: Mcp9808<'a, SensorI2C>,
    ) -> Result<Self> {
        Ok(Self {
            fan,
            curve: FanCurve::new()?,
            sensor,
        })
    }

    pub async fn update(&mut self) -> Result<()> {
        let temp = self.sensor.read_temp().await?;
        let speed = self.curve.sample(temp)?;
        self.fan.set_fan_speed(&speed);
        Ok(())
    }
}
