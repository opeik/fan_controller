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

/// Represents a DHT11 sensor.
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

/// Represents the current sensor state.
#[derive(Default, Debug, PartialEq)]
pub struct Data {
    /// Current humidity.
    pub humidity: Ratio,
    /// Current temperature.
    pub temperature: ThermodynamicTemperature,
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

    /// Reads the current state of the sensor.
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

    /// Reads the state of the sensor.
    async fn read_data(&mut self) -> Result<Data, HalError> {
        let mut state = bitarr![u8, Msb0; 0; 40];
        for mut bit in state.iter_mut() {
            *bit = self.read_bit().await?;
        }
        parse::<HalError>(state.as_bitslice())
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

/// Parses a DHT11 payload.
fn parse<HalError>(payload: &BitSlice<u8, Msb0>) -> Result<Data, HalError> {
    // A DHT payload is formatted as follows, the most significant bit is first:
    //
    //  0                   1
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  Humidity int | Humidity frac |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |    Temp int   |   Temp frac   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |    Checksum   |
    // +-+-+-+-+-+-+-+-+
    //
    // where:
    // - Humidity integral
    //     - 8 bits
    //     - The integral component of the relative humidity
    // - Humidity fractional
    //     - 8 bits
    //     - The decimal component of the relative humidity
    // - Temperature integral
    //     - 8 bits
    //     - The integral component of the temperature
    // - Temperature fractional
    //     - 8 bits
    //     - The decimal component of the temperature
    // - Checksum
    //     - 8 bits
    //     - Equal to the sum of the rest of the payload
    //
    // See: datasheet § 5.

    let expected_checksum = payload[32..40].load_be::<u8>();
    let actual_checksum = payload[0..32]
        .chunks(8)
        .fold(0u8, |sum, v| sum.wrapping_add(v.load_be::<u8>()));

    if actual_checksum != expected_checksum {
        return Err(Error::ChecksumMismatch {
            actual: actual_checksum,
            expected: expected_checksum,
        });
    }

    let humidity = i16_fixed_to_f32(&payload[0..16]);
    let temperature = i16_fixed_to_f32(&payload[16..32]);

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

/// Converts a fixed point [`i16`] to a [`f32`] given the format:
///
/// ```txt
///  0                   1
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |    Integral   |   Fractional  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
fn i16_fixed_to_f32(x: &BitSlice<u8, Msb0>) -> f32 {
    let is_signed = x[0];
    let sign = if is_signed { -1i8 } else { 1i8 };
    let magnitude = x[1..8].load_be::<u8>();
    (sign * (magnitude as i8)) as f32
}

#[cfg(test)]
mod tests {
    use core::convert::Infallible;

    use super::*;

    #[test]
    fn typical_payload_positive_temp() {
        assert_eq!(
            parse::<Infallible>([0x27, 0x00, 0x14, 0x00, 0x3b].view_bits()).unwrap(),
            Data {
                humidity: Ratio::new::<percent>(39.0),
                temperature: ThermodynamicTemperature::new::<degree_celsius>(20.0),
            }
        );
    }

    #[test]
    fn typical_payload_negative_temp() {
        assert_eq!(
            parse::<Infallible>([0x27, 0x00, 0x94, 0x00, 0xbb].view_bits()).unwrap(),
            Data {
                humidity: Ratio::new::<percent>(39.0),
                temperature: ThermodynamicTemperature::new::<degree_celsius>(-20.0),
            }
        );
    }

    #[test]
    fn checksum_mismatch() {
        assert_eq!(
            parse::<Infallible>([0x27, 0x00, 0x14, 0x00, 0xff].view_bits()),
            Err(Error::ChecksumMismatch {
                expected: 0xff,
                actual: 0x3b
            })
        );
    }

    #[test]
    fn invalid_humidity() {
        assert_eq!(
            parse::<Infallible>([0x00, 0x00, 0x14, 0x00, 0x14].view_bits()),
            Ok(Data {
                humidity: Ratio::new::<percent>(0.0),
                temperature: ThermodynamicTemperature::new::<degree_celsius>(20.0),
            })
        );

        assert_eq!(
            parse::<Infallible>([0x64, 0x00, 0x14, 0x00, 0x78].view_bits()),
            Ok(Data {
                humidity: Ratio::new::<percent>(100.0),
                temperature: ThermodynamicTemperature::new::<degree_celsius>(20.0),
            })
        );

        assert_eq!(
            parse::<Infallible>([0x65, 0x00, 0x14, 0x00, 0x79].view_bits()),
            Err(Error::InvalidHumidity(101.0))
        );
    }
}
