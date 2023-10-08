use driver::{self, Fan, Mcp9808};
use embassy_rp::{i2c, pwm};
use fan_controller::fan_curve::{self, FanCurve};

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

#[derive(derive_builder::Builder)]
#[builder(no_std, pattern = "owned")]
pub struct FanControl<'a, C: pwm::Channel, T: pwm::Channel, S: i2c::Instance> {
    fan: Fan<'a, C, T>,
    sensor: Mcp9808<'a, S>,
    #[builder(default)]
    curve: FanCurve,
}

impl<'a, C: pwm::Channel, T: pwm::Channel, S: i2c::Instance> FanControl<'a, C, T, S> {
    #[must_use]
    pub fn builder() -> FanControlBuilder<'a, C, T, S> {
        FanControlBuilder::default()
    }

    pub async fn update(&mut self) -> Result<()> {
        let temp = self.sensor.temp().await?;
        let speed = self.curve.sample(temp)?;
        self.fan.set_fan_speed(&speed);
        Ok(())
    }
}
