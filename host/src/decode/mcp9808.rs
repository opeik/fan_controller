use core::ops::RangeFrom;

use bitvec::{field::BitField, prelude::*, slice::BitSlice};
use fixed::FixedI16;
use heapless::Vec;
use uom::si::thermodynamic_temperature::degree_celsius;

use crate::units::ThermodynamicTemperature;

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a MCP9808 error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("heapless error")]
    HeaplessError,
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

/// Decodes a raw [`mcp9808`] payload.
pub fn decode(payload: &Payload) -> Result<Data> {
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
    const PAYLOAD_SIZE: usize = 2;
    const SIGN_BIT: usize = 4;
    const NUMERIC_BITS: RangeFrom<usize> = 5..;

    let mut bits = BitArray::<[u8; PAYLOAD_SIZE], Msb0>::ZERO;
    bits.set(0, payload[SIGN_BIT]);
    bits[NUMERIC_BITS].copy_from_bitslice(&payload[NUMERIC_BITS]);

    let bytes = bits
        .chunks(8)
        .map(BitField::load_be)
        .collect::<Vec<_, PAYLOAD_SIZE>>()
        .into_array()
        .map_err(|_| Error::HeaplessError)?;

    Ok(RawTemp::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn test_decode_raw_temp() -> Result<()> {
        let payload = [0b1100_0001_u8, 0b1001_0101u8];
        let raw_temp = decode_raw_temp(BitSlice::<_, Msb0>::from_slice(&payload))?;
        assert_eq!(raw_temp, RawTemp::from_num(25.3));

        Ok(())
    }
}
