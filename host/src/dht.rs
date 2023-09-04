//! DHT11 temperature and humidity sensor driver.
//!
//! # Protocol
//!
//! The protocol for requesting data from the DHT11 is as follows:
//!
//! ```txt
//!                                             DATA
//!                                ┌─────────────────────────────┐
//!     SYN    ACK       READY       SOT   0 BIT   SOT   1 BIT
//!    ┌────┐ ┌────┐ ┌───────────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐
//! ──┐      ┌──────┐      ┌──────┐      ┌──────┐      ┌──────┐
//!   │      │      │      │      │      │      │      │      │
//!   └──────┘      └──────┘      └──────┘      └──────┘      └──
//!     18ms   40μs   80μs   80μs   50μs   30μs   50μs   70μs
//! ```
//!
//! - SYN: Request to synchronize by pulling the data pin low for 18ms
//! - ACK: The DHT11 acknowledges the SYN by pulling the data pin up for 40μs
//! - READY: The DHT11 signals it's ready by pulling the data pin low then high for 80μs each
//! - SOT: The DHT11 signals the start of transmission by pulling the data pin low for 50μs, then:
//!     - A high pulse of 30μs indicates a 0 bit
//!     - A high pulse of 70μs indicates a 1 bit
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
use defmt::debug;
use embedded_hal::digital::{InputPin, OutputPin, PinState};
use embedded_hal_async::{delay::DelayUs as HalDelay, digital::Wait};
use uom::si::{
    f32::{Ratio, ThermodynamicTemperature},
    ratio::percent,
    thermodynamic_temperature::degree_celsius,
};

use crate::future::{timed, timeout};

type Result<T, E> = core::result::Result<T, Error<E>>;

/// Represents a sensor error.
#[derive(Debug, PartialEq, thiserror::Error, defmt::Format)]
pub enum Error<HalError> {
    /// The sensor is not present.
    #[error("sensor not present")]
    NotPresent,
    /// The read timed out.
    #[error("read timed out")]
    Timeout,
    /// The checksum is mismatched.
    #[error(
        "checksum mismatched (expected {:#0x}, found {:#0x})",
        expected,
        actual
    )]
    ChecksumMismatch { expected: u8, actual: u8 },
    #[error("invalid temperature, (expected -50≤x≤50°C, got {0}°C)")]
    InvalidTemperature(f32),
    #[error("invalid humidity, (expected 0≤x≤100%, got {0}%)")]
    InvalidHumidity(f32),
    #[error("hardware error")]
    HardwareError(#[from] HalError),
}

/// Represents a DHT11 temperature and humidity sensor.
///
/// See: [the datasheet].
///
/// [the datasheet]: https://www.mouser.com/datasheet/2/758/DHT11-Technical-Data-Sheet-Translated-Version-1143054.pdf
pub struct Dht11<Pin, DebugPin, Delay, HalError>
where
    Pin: InputPin<Error = HalError> + OutputPin<Error = HalError> + Wait,
    DebugPin: OutputPin<Error = HalError>,
    Delay: HalDelay,
{
    pin: Pin,
    delay: Delay,
    debug_pin: DebugPin,
}

/// Represents [`Dht11`] sensor data.
#[derive(Default, Debug, PartialEq)]
pub struct Data {
    /// Current humidity.
    pub humidity: Ratio,
    /// Current temperature.
    pub temperature: ThermodynamicTemperature,
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
            "Data {{ temperature: {}°C, humidity: {}% }}",
            self.temperature.get::<degree_celsius>(),
            self.humidity.get::<percent>(),
        )
    }
}

impl<Pin, DebugPin, Delay, HalError> Dht11<Pin, DebugPin, Delay, HalError>
where
    Pin: InputPin<Error = HalError> + OutputPin<Error = HalError> + Wait,
    DebugPin: OutputPin<Error = HalError>,
    Delay: HalDelay,
{
    /// Creates a new [`Dht11`].
    pub fn new(pin: Pin, delay: Delay, debug_pin: DebugPin) -> Self {
        Dht11 {
            pin,
            delay,
            debug_pin,
        }
    }

    /// Reads data from the sensor.
    pub async fn read(&mut self) -> Result<Data, HalError> {
        debug!("waking dht11...");
        self.wake().await?;

        debug!("connecting to dht11...");
        self.connect().await?;

        debug!("reading from dht11...");
        self.read_data().await
    }

    /// Wakes the sensor up.
    async fn wake(&mut self) -> Result<(), HalError> {
        // See: Datasheet § 5.2; figure 2.
        self.pin.set_low()?;
        self.delay.delay_ms(30).await;

        self.pin.set_high()?;
        self.delay.delay_us(40).await;

        Ok(())
    }

    /// Opens a connection to the sensor.
    async fn connect(&mut self) -> Result<(), HalError> {
        // See: datasheet § 5.2-3; figure 3.
        let timeout = 80 + 5;
        self.wait_for(PinState::High, timeout).await?;
        self.wait_for(PinState::Low, timeout).await?;
        Ok(())
    }

    /// Implements reading data from the sensor.
    async fn read_data(&mut self) -> Result<Data, HalError> {
        let mut data = bitarr![u8, Msb0; 0; 40];
        for mut bit in data.iter_mut() {
            *bit = self.read_bit().await?;
        }

        parse::<HalError>(data.as_bitslice())
    }

    /// Reads a bit of data from the sensor.
    async fn read_bit(&mut self) -> Result<bool, HalError> {
        // See: datasheet § 5.3; figure 4.
        self.wait_for(PinState::High, 55).await?;

        self.debug_pin.set_high()?;
        let (result, duration) = timed!(self.wait_for(PinState::Low, 70));
        self.debug_pin.set_low()?;
        result?;

        // A high level of ~30μ indicates a `0` bit, 70μ indicates a `1` bit.
        if duration.as_micros() > 30 + 20 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Waits for a pin state until the timeout.
    async fn wait_for(&mut self, state: PinState, timeout: u32) -> Result<(), HalError> {
        let timeout = self.delay.delay_us(timeout);
        match state {
            PinState::Low => timeout!(self.pin.wait_for_low(), timeout).transpose()?,
            PinState::High => timeout!(self.pin.wait_for_high(), timeout).transpose()?,
        };
        Ok(())
    }
}

/// Parses a [`Dht11`] payload.
fn parse<HalError>(data: &BitSlice<u8, Msb0>) -> Result<Data, HalError> {
    let RawData {
        humidity,
        humidity_frac,
        temperature,
        temperature_frac,
    } = parse_raw(data)?;

    let humidity = fixed_to_f32([humidity, humidity_frac].view_bits());
    let temperature = fixed_to_f32([temperature, temperature_frac].view_bits());

    if !(0.0..=100.0).contains(&humidity) {
        return Err(Error::InvalidHumidity(humidity));
    }

    if !(-50.0..=50.0).contains(&temperature) {
        return Err(Error::InvalidTemperature(temperature));
    }

    Ok(Data {
        humidity: Ratio::new::<percent>(humidity),
        temperature: ThermodynamicTemperature::new::<degree_celsius>(temperature),
    })
}

/// Parses a [`Dht11`] payload into a raw representation.
fn parse_raw<HalError>(data: &BitSlice<u8, Msb0>) -> Result<RawData, HalError> {
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
fn fixed_to_f32(x: &BitSlice<u8, Msb0>) -> f32 {
    let is_signed = x[0];
    let integer = x[1..8].load_be::<u8>() as f32;
    let fractional = x[8..16].load_be::<u8>() as f32;
    let sign = if is_signed { -1.0 } else { 1.0 };
    sign * (integer + (fractional / 10.0))
}

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use anyhow::Result;
    use float_eq::assert_float_eq;

    use super::*;

    type E = Infallible;

    /// Converts a [`i8`] to a binary [`u8`].
    fn from_i8(x: i8) -> u8 {
        let mut integer = x.abs().to_be() as u8;
        let bits = integer.view_bits_mut::<Msb0>();
        if x.is_negative() {
            bits.set(0, true);
        }

        integer
    }

    #[test]
    fn typical_positive_temp() -> Result<()> {
        let payload = [0x27, 0x00, 0x14, 0x08, 0x43].view_bits();
        let raw_data = parse_raw::<E>(payload)?;
        assert_eq!(raw_data.humidity, from_i8(39));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(20));
        assert_eq!(raw_data.temperature_frac, 8);

        let data = parse::<E>(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 39.0, ulps <= 10);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 20.8, ulps <= 10);

        Ok(())
    }

    #[test]
    fn typical_negative_temp() -> Result<()> {
        let payload = [0x27, 0x00, 0x94, 0x00, 0xbb].view_bits();
        let raw_data = parse_raw::<E>(payload)?;
        assert_eq!(raw_data.humidity, from_i8(39));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(-20));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = parse::<E>(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 39.0, ulps <= 10);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), -20.0, ulps <= 10);

        Ok(())
    }

    #[test]
    fn checksum_mismatch() {
        assert_eq!(
            parse::<E>([0x27, 0x00, 0x14, 0x00, 0x00].view_bits()),
            Err(Error::ChecksumMismatch {
                expected: 0x00,
                actual: 0x3b
            })
        );

        assert_eq!(
            parse::<E>([0x27, 0x00, 0x14, 0x00, 0xff].view_bits()),
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
        let raw_data = parse_raw::<E>(payload)?;
        assert_eq!(raw_data.humidity, from_i8(0));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(20));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = parse::<E>(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 0.0, ulps <= 10);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 20.0, ulps <= 10);

        // Check the upper boundary: 100%.
        let payload = [0x64, 0x00, 0x14, 0x00, 0x78].view_bits();
        let raw_data = parse_raw::<E>(payload)?;
        assert_eq!(raw_data.humidity, from_i8(100));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(20));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = parse::<E>(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 100.0, ulps <= 10);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 20.0, ulps <= 10);

        // Check below the lower boundary.
        assert_eq!(
            parse::<E>([0x80, 0x01, 0x14, 0x00, 0x95].view_bits()),
            Err(Error::InvalidHumidity(-0.1))
        );

        // Check above the upper boundary.
        assert_eq!(
            parse::<E>([0x64, 0x01, 0x14, 0x00, 0x79].view_bits()),
            Err(Error::InvalidHumidity(100.1))
        );

        Ok(())
    }
    #[test]
    fn invalid_temperature() -> Result<()> {
        // Check the lower boundary: -50.0°C.
        let payload = [0x14, 0x00, 0xB2, 0x00, 0xc6].view_bits();
        let raw_data = parse_raw::<E>(payload)?;
        assert_eq!(raw_data.humidity, from_i8(20));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(-50));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = parse::<E>(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 20.0, ulps <= 10);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), -50.0, ulps <= 10);

        // Check the upper boundary: 50.0°C.
        let payload = [0x14, 0x00, 0x32, 0x00, 0x46].view_bits();
        let raw_data = parse_raw::<E>(payload)?;
        assert_eq!(raw_data.humidity, from_i8(20));
        assert_eq!(raw_data.humidity_frac, 0);
        assert_eq!(raw_data.temperature, from_i8(50));
        assert_eq!(raw_data.temperature_frac, 0);

        let data = parse::<E>(payload)?;
        assert_float_eq!(data.humidity.get::<percent>(), 20.0, ulps <= 10);
        assert_float_eq!(data.temperature.get::<degree_celsius>(), 50.0, ulps <= 10);

        // Check below the lower boundary.
        assert_eq!(
            parse::<E>([0x14, 0x00, 0xB2, 0x01, 0xc7].view_bits()),
            Err(Error::InvalidTemperature(-50.1))
        );

        // Check above the upper boundary.
        assert_eq!(
            parse::<E>([0x14, 0x00, 0x32, 0x01, 0x47].view_bits()),
            Err(Error::InvalidTemperature(50.1))
        );

        Ok(())
    }
}
