use enterpolation::{
    bspline::{BSpline, BSplineError},
    Clamp, ConstSpace, Curve, Generator, Sorted,
};
use heapless::Vec;
use uom::si::{ratio::percent, thermodynamic_temperature::degree_celsius};

use crate::{
    decode::fan,
    units::{Ratio, ThermodynamicTemperature},
};

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a fan control error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    /// A decode error occurred.
    #[error("spline error: {0}")]
    SplineError(#[defmt(Debug2Format)] BSplineError),
    /// A fan decode error occurred.
    #[error("fan decode error: {0}")]
    FanDecodeError(#[from] fan::Error),
    #[error("heapless error")]
    HeaplessError,
}

const CURVE_SIZE: usize = 2;
type T = f64;
type Knots = Sorted<[T; CURVE_SIZE]>;
type Elements = [T; CURVE_SIZE];
type Space = ConstSpace<T, CURVE_SIZE>;

pub struct FanCurve(Clamp<BSpline<Knots, Elements, Space>>);

impl FanCurve {
    pub fn new() -> Result<Self> {
        let curve = Self::default_curve()?;
        let (temps, fan_powers) = Self::unzip_curve(curve);
        let curve = BSpline::builder()
            .elements(fan_powers.into_array().map_err(|_| Error::HeaplessError)?)
            .knots(temps.into_array().map_err(|_| Error::HeaplessError)?)
            .constant::<CURVE_SIZE>()
            .build()
            .map_err(Error::SplineError)?
            .clamp();
        Ok(Self(curve))
    }

    fn default_curve() -> Result<Vec<(ThermodynamicTemperature, fan::Power), CURVE_SIZE>> {
        [(20.0, 30.0), (65.0, 100.0)]
            .into_iter()
            .map(|(temp, fan_power)| {
                Ok((
                    ThermodynamicTemperature::new::<degree_celsius>(temp),
                    fan::Power::new(Ratio::new::<percent>(fan_power))?,
                ))
            })
            .collect::<Result<Vec<_, CURVE_SIZE>>>()
    }

    fn unzip_curve<T>(collection: T) -> (Vec<f64, CURVE_SIZE>, Vec<f64, CURVE_SIZE>)
    where
        T: IntoIterator<Item = (ThermodynamicTemperature, fan::Power)>,
    {
        collection
            .into_iter()
            .map(|(temp, fan_power)| (temp.get::<degree_celsius>(), fan_power.get::<percent>()))
            .unzip()
    }

    pub fn sample(&self, temp: ThermodynamicTemperature) -> Result<fan::Power> {
        Ok(fan::Power::new(Ratio::new::<percent>(
            self.0.gen(temp.get::<degree_celsius>()),
        ))?)
    }
}

impl From<BSplineError> for Error {
    fn from(value: BSplineError) -> Self {
        Error::SplineError(value)
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use anyhow::{Context, Result};
    use float_eq::assert_float_eq;
    use uom::si::ratio::ratio;

    use super::*;

    #[test]
    fn sample_curve() -> Result<()> {
        let curve = FanCurve::new()?;

        let actual_samples = (0..=100)
            .step_by(10)
            .map(|x| ThermodynamicTemperature::new::<degree_celsius>(f64::from(x)))
            .map(|x| curve.sample(x).context("oops"))
            .collect::<Result<Vec<_>>>()?;

        let expected_samples = [
            0.3,
            0.3,
            0.3,
            0.455_555_555_555_555_6,
            0.611_111_111_111_111_2,
            0.766_666_666_666_666_6,
            0.922_222_222_222_222_2,
            1.0,
            1.0,
            1.0,
        ];

        for (actual, expected) in actual_samples.into_iter().zip(expected_samples) {
            assert_float_eq!(actual.get::<ratio>(), expected, ulps <= 4);
        }

        Ok(())
    }
}
