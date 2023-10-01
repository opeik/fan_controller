use core::ops::RangeInclusive;

use bitvec::{prelude::*, slice::BitSlice};
use fixed::FixedI16;
use uom::si::thermodynamic_temperature::degree_celsius;

use crate::units::ThermodynamicTemperature;

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a MCP9808 error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("invalid temperature: expected –40°C≤x≤125°C, got {0}°C")]
    InvalidTemp(f64),
}

/// Represents MCP9808 data.
#[derive(Debug, Copy, Clone, defmt::Format)]
pub enum Data {
    Temp(#[defmt(Debug2Format)] ThermodynamicTemperature),
}

type RawTemp = FixedI16<4>;

/// Represents raw MCP9808 data.
#[derive(Debug, Copy, Clone)]
enum RawData {
    Temp(RawTemp),
}

/// Represents a MCP9808 payload.
pub enum Payload<'a> {
    Temp(&'a BitSlice<u8, Msb0>),
}

const PAYLOAD_SIZE: usize = 2;

/// Decodes a raw [`mcp9808`] payload.
pub fn decode(payload: &Payload) -> Result<Data> {
    // –40°C to +125°C
    Ok(match decode_raw(payload)? {
        RawData::Temp(v) => Data::Temp(ThermodynamicTemperature::new::<degree_celsius>(v.to_num())),
    })
}

/// Decodes a raw [`mcp9808`] payload.
fn decode_raw(payload: &Payload) -> Result<RawData> {
    Ok(match payload {
        Payload::Temp(v) => RawData::Temp(decode_raw_temp(v)?),
    })
}

fn decode_raw_temp(payload: &BitSlice<u8, Msb0>) -> Result<RawTemp> {
    const SIGN_BIT: usize = 3;
    const NUMERIC_BITS: RangeInclusive<usize> = 4..=15;

    let sign = if payload[SIGN_BIT] { -1 } else { 1 };
    let mut bits = BitArray::<[u8; PAYLOAD_SIZE], Msb0>::ZERO;
    bits[NUMERIC_BITS].copy_from_bitslice(&payload[NUMERIC_BITS]);
    let temp = RawTemp::from_be_bytes(bits.into_inner()) * sign;

    if !(-40.0..=125.0).contains(&temp.to_num::<f64>()) {
        return Err(Error::InvalidTemp(temp.to_num()));
    }

    Ok(temp)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decode_raw {
        use super::*;

        fn test_decode(payload: [u8; 2], raw_temp: RawTemp) {
            assert_eq!(
                raw_temp,
                decode_raw_temp(BitSlice::<_, Msb0>::from_slice(&payload)).unwrap()
            );
        }

        #[test]
        fn zero_celsius() {
            test_decode([0b0000_0000, 0b0000_0000], RawTemp::from_num(0.0));
        }

        #[test]
        fn slightly_above_zero_celsius() {
            test_decode([0b0000_0000, 0b0000_0001], RawTemp::from_num(0.062));
            test_decode([0b0000_0000, 0b0000_0010], RawTemp::from_num(0.125));
            test_decode([0b0000_0000, 0b0000_1000], RawTemp::from_num(0.500));
            test_decode([0b0000_0000, 0b0000_1010], RawTemp::from_num(0.625));
        }

        #[test]
        fn slightly_below_zero_celsius() {
            test_decode([0b0001_0000, 0b0000_0001], RawTemp::from_num(-0.062));
            test_decode([0b0001_0000, 0b0000_0010], RawTemp::from_num(-0.125));
            test_decode([0b0001_0000, 0b0000_1000], RawTemp::from_num(-0.500));
            test_decode([0b0001_0000, 0b0000_1010], RawTemp::from_num(-0.625));
        }

        #[test]
        fn above_zero_celsius() {
            test_decode([0b0000_0001, 0b1001_0100], RawTemp::from_num(25.250));
        }

        #[test]
        fn below_zero_celsius() {
            test_decode([0b0001_0001, 0b1001_0100], RawTemp::from_num(-25.250));
        }
    }
}
