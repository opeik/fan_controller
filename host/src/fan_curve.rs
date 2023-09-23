use defmt::info;
use enterpolation::{
    bspline::{BSpline, BSplineError},
    Clamp, ConstSpace, Curve, Generator, Sorted,
};
use heapless::Vec;

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a fan control error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    /// A decode error occurred.
    #[error("spline error: {0}")]
    SplineError(#[defmt(Debug2Format)] BSplineError),
}

#[derive(derive_more::Deref)]
struct FanCurve(Clamp<BSpline<Sorted<[f64; 3]>, [f64; 3], ConstSpace<f64, 3>>>);

impl FanCurve {
    pub fn new() -> Result<Self> {
        let x = BSpline::builder()
            .elements([0.0, 30.0, 100.0])
            .knots([0.0, 20.0, 65.0])
            .constant::<3>()
            .build()
            .map_err(Error::SplineError)?
            .clamp();
        Ok(Self(x))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pain() {
        let x = FanCurve::new().unwrap();
    }
}
