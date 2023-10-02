use uom::si::thermodynamic_temperature::degree_celsius;

use crate::units::ThermodynamicTemperature;

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a MCP9808 decoding error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("invalid temperature: expected –40°C≤x≤125°C, got {0}°C")]
    InvalidTemperature(f64),
}

/// Represents MCP9808 data.
#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum Data {
    Temperature(#[defmt(Debug2Format)] ThermodynamicTemperature),
}

pub mod raw {
    use core::ops::RangeInclusive;

    use bitvec::prelude::*;
    use fixed::FixedI16;

    use super::{Error, Result};

    pub type Temperature = FixedI16<4>;
    pub type TemperaturePayload = BitArray<[u8; 2], Msb0>;

    /// Represents raw MCP9808 data.
    #[derive(Debug, Copy, Clone)]
    pub enum Data {
        Temperature(Temperature),
    }

    /// Represents a MCP9808 payload.
    #[derive(Debug, Copy, Clone)]
    pub enum Payload {
        Temperature(TemperaturePayload),
    }

    /// Decodes a raw MCP9808 payload.
    pub fn decode(payload: Payload) -> Result<Data> {
        Ok(match payload {
            Payload::Temperature(v) => Data::Temperature(decode_temperature(v)?),
        })
    }

    /// Decodes a raw MCP9808 temperature payload.
    pub fn decode_temperature(payload: TemperaturePayload) -> Result<Temperature> {
        const SIGN_BIT: usize = 3;
        const NUMERIC_BITS: RangeInclusive<usize> = 4..=15;

        let sign = if payload[SIGN_BIT] { -1 } else { 1 };
        let mut bits = TemperaturePayload::ZERO;
        bits[NUMERIC_BITS].copy_from_bitslice(&payload[NUMERIC_BITS]);
        let temp = Temperature::from_be_bytes(bits.into_inner()) * sign;

        if !(-40.0..=125.0).contains(&temp.to_num::<f64>()) {
            return Err(Error::InvalidTemperature(temp.to_num()));
        }

        Ok(temp)
    }
}

/// Decodes a raw MCP9808 payload.
pub fn decode(payload: raw::Payload) -> Result<Data> {
    Ok(match raw::decode(payload)? {
        raw::Data::Temperature(v) => {
            Data::Temperature(ThermodynamicTemperature::new::<degree_celsius>(v.to_num()))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod temp {
        use super::*;

        fn assert_temp_eq(payload: [u8; 2], raw_temp: raw::Temperature) {
            assert_eq!(
                raw_temp,
                raw::decode_temperature(raw::TemperaturePayload::from(payload)).unwrap()
            );
        }

        #[test]
        fn zero_celsius() {
            assert_temp_eq([0b0000_0000, 0b0000_0000], raw::Temperature::from_num(0.0));
        }

        #[test]
        fn slightly_above_zero_celsius() {
            assert_temp_eq(
                [0b0000_0000, 0b0000_0001],
                raw::Temperature::from_num(0.062),
            );
            assert_temp_eq(
                [0b0000_0000, 0b0000_0010],
                raw::Temperature::from_num(0.125),
            );
            assert_temp_eq(
                [0b0000_0000, 0b0000_1000],
                raw::Temperature::from_num(0.500),
            );
            assert_temp_eq(
                [0b0000_0000, 0b0000_1010],
                raw::Temperature::from_num(0.625),
            );
        }

        #[test]
        fn slightly_below_zero_celsius() {
            assert_temp_eq(
                [0b0001_0000, 0b0000_0001],
                raw::Temperature::from_num(-0.062),
            );
            assert_temp_eq(
                [0b0001_0000, 0b0000_0010],
                raw::Temperature::from_num(-0.125),
            );
            assert_temp_eq(
                [0b0001_0000, 0b0000_1000],
                raw::Temperature::from_num(-0.500),
            );
            assert_temp_eq(
                [0b0001_0000, 0b0000_1010],
                raw::Temperature::from_num(-0.625),
            );
        }

        #[test]
        fn above_zero_celsius() {
            assert_temp_eq(
                [0b0000_0001, 0b1001_0100],
                raw::Temperature::from_num(25.250),
            );
        }

        #[test]
        fn below_zero_celsius() {
            assert_temp_eq(
                [0b0001_0001, 0b1001_0100],
                raw::Temperature::from_num(-25.250),
            );
        }
    }
}
