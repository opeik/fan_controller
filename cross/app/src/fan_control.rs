use defmt::info;
use driver::{self, Fan, Mcp9808};
use embassy_rp::{i2c, pwm};
use fan_controller::fan_curve::{self, FanCurve};
use uom::si::{frequency::hertz, ratio::percent, thermodynamic_temperature::degree_celsius};

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
pub struct FanControl<'a, C: pwm::Channel, S: i2c::Instance> {
    fan: Fan<'a, C>,
    sensor: Mcp9808<'a, S>,
    #[builder(default)]
    curve: FanCurve,
}

impl<'a, C: pwm::Channel, S: i2c::Instance> FanControl<'a, C, S> {
    #[must_use]
    pub fn builder() -> FanControlBuilder<'a, C, S> {
        FanControlBuilder::default()
    }

    pub async fn update(&mut self) -> Result<()> {
        let temp = self.sensor.temp().await?;
        info!("temp: {}°C", temp.get::<degree_celsius>());
        let target_speed = self.curve.sample(temp)?;
        info!("new fan speed: {}%", target_speed.get::<percent>());
        // let current_freq = self.fan.fan_freq().await?;
        // info!("current fan freq: {}Hz", current_freq.get::<hertz>());
        self.fan.set_fan_speed(&target_speed);
        Ok(())
    }
}
