use bitvec::{prelude::Msb0, BitArr};
use defmt::debug;
use embassy_time::Instant;
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
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error<HalError> {
    /// The sensor is not present.
    #[error("sensor not present")]
    NotPresent,
    /// The read timed out.
    #[error("read timed out")]
    Timeout,
    /// The state checksum is mismatched.
    #[error("checksum mismatched (expected {expected}, found {actual})")]
    ChecksumMismatch { expected: u8, actual: u8 },
    #[error("invalid temperature, (expected 0≤x≤50°C, got {0}°C)")]
    InvalidTemperature(f32),
    #[error("invalid humidity, (expected 0≤x≤100%, got {0}%)")]
    InvalidHumidity(f32),
    #[error("hardware error")]
    HardwareError(#[from] HalError),
}

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
pub struct State {
    /// Current temperature.
    pub temperature: ThermodynamicTemperature,
    /// Current humidity.
    pub humidity: Ratio,
}

type RawState = BitArr!(for 5 * 8, in u32, Msb0);

impl defmt::Format for State {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "State {{ temperature: {}°C, humidity: {}% }}",
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
    ///
    /// See: the [datasheet].
    ///
    /// [datasheet]: https://www.mouser.com/datasheet/2/758/DHT11-Technical-Data-Sheet-Translated-Version-1143054.pdf
    pub async fn read(&mut self) -> Result<State, HalError> {
        debug!("waking dht11...");
        self.wake().await?;
        debug!("connecting to dht11...");
        self.connect().await?;
        debug!("reading dht11 state...");
        self.read_state().await
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
    async fn read_state(&mut self) -> Result<State, HalError> {
        let bytes = self.read_state_raw().await?;

        for byte in bytes {
            debug!("read byte: {:08b}", byte);
        }

        parse_state::<HalError>(&bytes)
    }

    async fn read_state_raw(&mut self) -> Result<RawState, HalError> {
        let mut bytes: RawState = [0; 5];
        for byte in bytes.iter_mut() {
            *byte = self.read_byte().await?;
        }
        debug!("state: {:?}", bytes);
        Ok(bytes)
    }

    // Reads a byte of data from the sensor.
    async fn read_byte(&mut self) -> Result<u8, HalError> {
        let mut byte: u8 = 0;
        for i in 0..8 {
            let bit_mask = 1 << (7 - (i % 8));
            if self.read_bit().await? {
                byte |= bit_mask;
            }
        }
        Ok(byte)
    }

    /// Reads a bit of data from the sensor.
    async fn read_bit(&mut self) -> Result<bool, HalError> {
        // See: datasheet § 5.3; figure 4.
        self.wait_for(PinState::High, 55).await?;

        self.debug_pin.set_high()?;
        let (result, duration) = timed!(self.wait_for(PinState::Low, 70));
        self.debug_pin.set_low()?;
        result?;

        // A high level of 26-28μ indicates a `0` bit, 70μ indicates a `1` bit.
        if duration.as_micros() > 30 + 20 {
            debug!("got 1 bit, duration: {}", duration.as_micros());
            Ok(true)
        } else {
            debug!("got 0 bit, duration: {}", duration.as_micros());
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

    async fn wait_for_exclusive(&mut self, state: PinState, timeout: u32) -> Result<(), HalError> {
        let timeout = self.delay.delay_us(timeout);
        match state {
            PinState::Low => timeout!(self.pin.wait_for_falling_edge(), timeout).transpose()?,
            PinState::High => timeout!(self.pin.wait_for_rising_edge(), timeout).transpose()?,
        };
        Ok(())
    }
}

fn parse_state<HalError>(bytes: &[u8]) -> Result<State, HalError> {
    // The DHT11 sends payloads in two's compliment, most significant bit first. A payload
    // is formatted as follows:
    //
    //          0                   1
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  Humidity int |  Humidity dec |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |Temperature int|Temperature dec|
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |    Checksum   |
    // +-+-+-+-+-+-+-+-+
    //
    // where:
    // - Humidity (integral)
    //     - 8 bits, signed
    //     - The integral component of the relative humidity
    // - Humidity (decimal)
    //     - 8 bits, unsigned
    //     - The decimal component of the relative humidity
    // - Temperature (integral)
    //     - 8 bits, signed
    //     - The integral component of the temperature
    // - Temperature (decimal)
    //     - 8 bits, unsigned
    //     - The decimal component of the temperature
    // - Checksum
    //     - 8 bits, unsigned
    //     - Should match the sum of all other bytes
    //
    // See: datasheet § 5.

    let expected_checksum = bytes[4];
    let actual_checksum = bytes[0..=3].iter().fold(0u8, |sum, v| sum.wrapping_add(*v));
    if expected_checksum != actual_checksum {
        return Err(Error::ChecksumMismatch {
            expected: expected_checksum,
            actual: actual_checksum,
        });
    }

    let humidity = bytes[0];
    let temp_signed = bytes[2];
    let temperature = {
        let (signed, magnitude) = convert_signed(temp_signed);
        let temp_sign = if signed { -1 } else { 1 };
        temp_sign * magnitude as i8
    };

    Ok(State {
        temperature: ThermodynamicTemperature::new::<degree_celsius>(temperature as f32),
        humidity: Ratio::new::<percent>(humidity as f32),
    })
}

fn convert_signed(x: u8) -> (bool, u8) {
    let sign = x & 0x80 != 0;
    let magnitude = x & 0x7F;
    (sign, magnitude)
}

#[cfg(test)]
mod test {
    use core::convert::Infallible;

    use super::*;

    #[test]
    fn test_read_state_raw() {
        assert_eq!(
            parse_state::<Infallible>(&[0x32, 0, 0x1B, 0, 0x4D]).unwrap(),
            State {
                temperature: ThermodynamicTemperature::new::<degree_celsius>(27.0),
                humidity: Ratio::new::<percent>(50.0),
            }
        );

        assert_eq!(
            parse_state::<Infallible>(&[0x80, 0, 0x83, 0, 0x3]).unwrap(),
            State {
                temperature: ThermodynamicTemperature::new::<degree_celsius>(-3.0),
                humidity: Ratio::new::<percent>(128.0),
            }
        );
    }
}
