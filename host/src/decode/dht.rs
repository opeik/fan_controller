//! DHT11 temperature and humidity sensor decoder.
//!
//! # Encoding
//!
//! A DHT11 payload is encoded as follows, the most significant bit is first:
//!
//! ```txt
//!  0                   1
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  Humidity int | Humidity frac |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |    Temp int   |   Temp frac   |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |    Checksum   |
//! +-+-+-+-+-+-+-+-+
//! ```
//!
//! where:
//! - Humidity integer (8 bits, signed)
//!     - The integer component of humidity
//! - Humidity fractional (8 bits, unsigned)
//!     - The decimal component of humidity
//! - Temperature integer (8 bits, signed)
//!     - The integer component of temperature
//! - Temperature fractional (8 bits, unsigned)
//!     - The decimal component of temperature
//! - Checksum (8 bits, unsigned)
//!     - Equal to the sum of the rest of the payload
//!
//! See: [datasheet] § 5.
//!
//! [datasheet]: https://www.mouser.com/datasheet/2/758/DHT11-Technical-Data-Sheet-Translated-Version-1143054.pdf
use bitvec::prelude::*;
use uom::si::{self, ratio::percent, thermodynamic_temperature::degree_celsius};

pub type Result<T> = core::result::Result<T, Error>;

/// Represents a sensor error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error {
    /// The checksum is mismatched.
    #[error(
        "checksum mismatched (expected {:#0x}, found {:#0x})",
        expected,
        actual
    )]
    ChecksumMismatch { expected: u8, actual: u8 },
    #[error("invalid temperature: expected -50≤x≤50°C, got {0}°C")]
    InvalidTemperature(f64),
    #[error("invalid humidity: expected 0≤x≤100%, got {0}%")]
    InvalidHumidity(f64),
}

/// Represents [`Dht11`] sensor data.
#[derive(Default, Debug, PartialEq)]
pub struct Data {
    /// Current humidity.
    pub humidity: si::f64::Ratio,
    /// Current temperature.
    pub temperature: si::f64::ThermodynamicTemperature,
}

/// Represents raw [`Dht11`] sensor data.
#[derive(Default, Debug, PartialEq)]
struct RawData {
    pub humidity: u8,
    pub humidity_frac: u8,
    pub temperature: u8,
    pub temperature_frac: u8,
}

impl defmt::Format for Data {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "Data {{ humidity: {}%, temperature: {}°C }}",
            self.humidity.get::<percent>(),
            self.temperature.get::<degree_celsius>(),
        );
    }
}

/// Decodes a [`Dht11`] payload.
pub fn decode(data: &BitSlice<u8, Msb0>) -> Result<Data> {
    let RawData {
        humidity,
        humidity_frac,
        temperature,
        temperature_frac,
    } = decode_raw(data)?;

    let humidity = fixed_to_f64([humidity, humidity_frac].view_bits());
    let temperature = fixed_to_f64([temperature, temperature_frac].view_bits());

    if !(0.0..=100.0).contains(&humidity) {
        return Err(Error::InvalidHumidity(humidity));
    }

    if !(-50.0..=50.0).contains(&temperature) {
        return Err(Error::InvalidTemperature(temperature));
    }

    Ok(Data {
        humidity: si::f64::Ratio::new::<percent>(humidity),
        temperature: si::f64::ThermodynamicTemperature::new::<degree_celsius>(temperature),
    })
}

/// Decodes a [`Dht11`] payload into a raw representation.
fn decode_raw(data: &BitSlice<u8, Msb0>) -> Result<RawData> {
    let expected_checksum = data[32..40].load_be::<u8>();
    let actual_checksum = data[0..32]
        .chunks(8)
        .fold(0u8, |sum, v| sum.wrapping_add(v.load_be::<u8>()));

    if actual_checksum != expected_checksum {
        return Err(Error::ChecksumMismatch {
            actual: actual_checksum,
            expected: expected_checksum,
        });
    }

    let humidity = data[0..8].load_be::<u8>();
    let humidity_frac = data[8..16].load_be::<u8>();
    let temperature = data[16..24].load_be::<u8>();
    let temperature_frac = data[24..32].load_be::<u8>();

    Ok(RawData {
        humidity,
        humidity_frac,
        temperature,
        temperature_frac,
    })
}

/// Converts a signed 16 bit fixed point number to a [`f32`] given the format:
///
/// ```txt
///   0                   1
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |    Integer    |   Fractional  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
fn fixed_to_f64(x: &BitSlice<u8, Msb0>) -> f64 {
    let is_signed = x[0];
    let integer = f64::from(x[1..8].load_be::<u8>());
    let fractional = f64::from(x[8..16].load_be::<u8>());
    let sign = if is_signed { -1.0 } else { 1.0 };
    sign * (integer + (fractional / 10.0))
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use float_eq::assert_float_eq;

    use super::*;

    /// Converts a [`i8`] to a binary [`u8`].
    fn from_i8(x: i8) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        let mut integer = x.abs().to_be() as u8;
        let bits = integer.view_bits_mut::<Msb0>();
        if x.is_negative() {
            bits.set(0, true);
        }

        integer
    }

    #[test]
    fn typical_positive_temp() -> Result<()> {
        let payload = [0x27, 0x03, 0x14, 0x08, 0x46].view_bits();
        let raw_data = decode_raw(payload)?;
        assert_eq!(raw_data.humidity, from_i8(39));
        assert_eq!(raw_data.humidity_frac, 3);
        assert_eq!(raw_data.temperature, from_i8(20));
        assert_eq!(raw_data.temperature_frac, 8);

        let data = decode(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 39.3, ulps <= 4);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 20.8, ulps <= 4);

        Ok(())
    }

    #[test]
    fn typical_negative_temp() -> Result<()> {
        let payload = [0x27, 0x03, 0x94, 0x08, 0xc6].view_bits();
        let raw_data = decode_raw(payload)?;
        assert_eq!(raw_data.humidity, from_i8(39));
        assert_eq!(raw_data.humidity_frac, 3);
        assert_eq!(raw_data.temperature, from_i8(-20));
        assert_eq!(raw_data.temperature_frac, 8);

        let data = decode(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 39.3, ulps <= 4);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), -20.8, ulps <= 4);

        Ok(())
    }

    #[test]
    fn checksum_mismatch() {
        assert_eq!(
            decode([0x27, 0x00, 0x14, 0x00, 0x00].view_bits()),
            Err(Error::ChecksumMismatch {
                expected: 0x00,
                actual: 0x3b
            })
        );

        assert_eq!(
            decode([0x27, 0x00, 0x14, 0x00, 0xff].view_bits()),
            Err(Error::ChecksumMismatch {
                expected: 0xff,
                actual: 0x3b
            })
        );
    }

    #[test]
    fn invalid_humidity() -> Result<()> {
        // Check the lower boundary: 0%.
        let payload = [0x00, 0x00, 0x14, 0x00, 0x14].view_bits();
        let raw_data = decode_raw(payload)?;
        assert_eq!(raw_data.humidity, from_i8(0));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(20));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = decode(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 0.0, ulps <= 4);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 20.0, ulps <= 4);

        // Check the upper boundary: 100%.
        let payload = [0x64, 0x00, 0x14, 0x00, 0x78].view_bits();
        let raw_data = decode_raw(payload)?;
        assert_eq!(raw_data.humidity, from_i8(100));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(20));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = decode(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 100.0, ulps <= 4);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 20.0, ulps <= 4);

        // Check below the lower boundary.
        assert_eq!(
            decode([0x80, 0x01, 0x14, 0x00, 0x95].view_bits()),
            Err(Error::InvalidHumidity(-0.1))
        );

        // Check above the upper boundary.
        assert_eq!(
            decode([0x64, 0x01, 0x14, 0x00, 0x79].view_bits()),
            Err(Error::InvalidHumidity(100.1))
        );

        Ok(())
    }
    #[test]
    fn invalid_temperature() -> Result<()> {
        // Check the lower boundary: -50.0°C.
        let payload = [0x14, 0x00, 0xB2, 0x00, 0xc6].view_bits();
        let raw_data = decode_raw(payload)?;
        assert_eq!(raw_data.humidity, from_i8(20));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(-50));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = decode(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 20.0, ulps <= 4);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), -50.0, ulps <= 4);

        // Check the upper boundary: 50.0°C.
        let payload = [0x14, 0x00, 0x32, 0x00, 0x46].view_bits();
        let raw_data = decode_raw(payload)?;
        assert_eq!(raw_data.humidity, from_i8(20));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(50));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = decode(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 20.0, ulps <= 4);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 50.0, ulps <= 4);

        // Check below the lower boundary.
        assert_eq!(
            decode([0x14, 0x00, 0xB2, 0x01, 0xc7].view_bits()),
            Err(Error::InvalidTemperature(-50.1))
        );

        // Check above the upper boundary.
        assert_eq!(
            decode([0x14, 0x00, 0x32, 0x01, 0x47].view_bits()),
            Err(Error::InvalidTemperature(50.1))
        );

        Ok(())
    }
}
