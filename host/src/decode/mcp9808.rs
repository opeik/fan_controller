use core::ops::RangeInclusive;

use bitvec::prelude::*;
use fixed::FixedI16;
use uom::si::thermodynamic_temperature::degree_celsius;

use crate::units::ThermodynamicTemperature;

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a MCP9808 decoding error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("invalid temperature: expected –40°C≤x≤125°C, got {0}°C")]
    InvalidTemperature(f64),
}

#[derive(Debug, PartialEq, Eq, derive_more::Deref, defmt::Format)]
pub struct Address(pub u16);

#[derive(Debug, PartialEq, Eq, derive_more::Deref, defmt::Format)]
pub struct ManufacturerId(pub u16);

#[derive(Debug, PartialEq, Eq, derive_more::Deref, defmt::Format)]
pub struct DeviceId(pub u8);

#[derive(Debug, PartialEq, Eq, derive_more::Deref, defmt::Format)]
pub struct Revision(pub u8);

pub type TemperaturePayload = BitArray<[u8; 2], Msb0>;
pub type ManufacturerIdPayload = BitArray<[u8; 2], Msb0>;
pub type DeviceIdPayload = BitArray<[u8; 2], Msb0>;

pub mod raw {
    use super::*;

    pub type Temperature = FixedI16<4>;

    /// Decodes a MCP9808 temperature payload.
    ///
    /// See: datasheet § 5.1.3, page 24.
    pub fn decode_temperature(payload: TemperaturePayload) -> Result<Temperature> {
        const SIGN_BIT: usize = 3;
        const NUMERIC_BITS: RangeInclusive<usize> = 4..=15;

        let sign = if payload[SIGN_BIT] { -1 } else { 1 };
        let mut bits = TemperaturePayload::ZERO;
        bits[NUMERIC_BITS].copy_from_bitslice(&payload[NUMERIC_BITS]);
        Ok(Temperature::from_be_bytes(bits.into_inner()) * sign)
    }

    /// Decodes a MCP9808 manufacturer ID payload.
    ///
    /// See: datasheet § 5.1.4, page 27.
    pub fn decode_manufacturer_id(payload: ManufacturerIdPayload) -> Result<ManufacturerId> {
        Ok(ManufacturerId(payload.load_be::<u16>()))
    }

    /// Decodes a MCP9808 device ID and revision payload.
    ///
    /// See: datasheet § 5.1.5, page 28.
    pub fn decode_device_id(payload: DeviceIdPayload) -> Result<(DeviceId, Revision)> {
        Ok((
            DeviceId(payload[0..8].load_be::<u8>()),
            Revision(payload[8..16].load_be::<u8>()),
        ))
    }
}

/// Decodes a MCP9808 temperature payload.
pub fn decode_temperature(payload: TemperaturePayload) -> Result<ThermodynamicTemperature> {
    let temp = raw::decode_temperature(payload)?.to_num::<f64>();

    if !(-40.0..=125.0).contains(&temp) {
        return Err(Error::InvalidTemperature(temp));
    }

    Ok(ThermodynamicTemperature::new::<degree_celsius>(temp))
}

/// Decodes a MCP9808 manufacturer ID payload.
pub fn decode_manufacturer_id(payload: TemperaturePayload) -> Result<ManufacturerId> {
    raw::decode_manufacturer_id(payload)
}

/// Decodes a MCP9808 device ID payload.
pub fn decode_device_id(payload: DeviceIdPayload) -> Result<(DeviceId, Revision)> {
    raw::decode_device_id(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    mod temp {
        use super::*;
        use crate::decode::mcp9808::{decode_temperature, TemperaturePayload};

        #[test]
        fn valid_range() {
            let temp = decode_temperature(TemperaturePayload::from([0x0, 0x0])).unwrap();
            assert_eq!(temp, ThermodynamicTemperature::new::<degree_celsius>(0.0));
        }

        #[test]
        fn invalid_range() {
            let temp = decode_temperature(TemperaturePayload::from([0x1F, 0xF0]));
            assert!(temp.is_err());
        }

        mod raw {
            use crate::decode::mcp9808::{
                raw::{self, Temperature},
                TemperaturePayload,
            };

            fn assert_temp_eq(payload: [u8; 2], raw_temp: Temperature) {
                assert_eq!(
                    raw_temp,
                    raw::decode_temperature(TemperaturePayload::from(payload)).unwrap()
                );
            }

            #[test]
            fn zero_celsius() {
                assert_temp_eq([0b0000_0000, 0b0000_0000], Temperature::from_num(0.0));
            }

            #[test]
            fn slightly_above_zero_celsius() {
                assert_temp_eq([0b0000_0000, 0b0000_0001], Temperature::from_num(0.062));
                assert_temp_eq([0b0000_0000, 0b0000_0010], Temperature::from_num(0.125));
                assert_temp_eq([0b0000_0000, 0b0000_1000], Temperature::from_num(0.500));
                assert_temp_eq([0b0000_0000, 0b0000_1010], Temperature::from_num(0.625));
            }

            #[test]
            fn slightly_below_zero_celsius() {
                assert_temp_eq([0b0001_0000, 0b0000_0001], Temperature::from_num(-0.062));
                assert_temp_eq([0b0001_0000, 0b0000_0010], Temperature::from_num(-0.125));
                assert_temp_eq([0b0001_0000, 0b0000_1000], Temperature::from_num(-0.500));
                assert_temp_eq([0b0001_0000, 0b0000_1010], Temperature::from_num(-0.625));
            }

            #[test]
            fn above_zero_celsius() {
                assert_temp_eq([0b0000_0001, 0b1001_0100], Temperature::from_num(25.250));
            }

            #[test]
            fn below_zero_celsius() {
                assert_temp_eq([0b0001_0001, 0b1001_0100], Temperature::from_num(-25.250));
            }
        }
    }

    mod manufacturer_id {
        mod raw {
            use crate::decode::mcp9808::{raw, DeviceId, DeviceIdPayload, Revision};

            #[test]
            fn standard() {
                let (device_id, revision) =
                    raw::decode_device_id(DeviceIdPayload::from([0x54, 0x0])).unwrap();
                assert_eq!(device_id, DeviceId(0x54));
                assert_eq!(revision, Revision(0x0));
            }
        }
    }
}
