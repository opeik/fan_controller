use bitvec::{array::BitArray, prelude::*, BitArr};
use defmt::{debug, info};
use embassy_time::Instant;
use embedded_hal::digital::{InputPin, OutputPin, PinState};
use embedded_hal_async::{delay::DelayUs as HalDelay, digital::Wait};
use futures::{
    future::{self, Either},
    pin_mut,
};
use uom::si::{
    f32::{Ratio, ThermodynamicTemperature, Time},
    ratio::{percent, ratio},
    thermodynamic_temperature::degree_celsius,
    time::microsecond,
};

use crate::future::{Timed, TimedExt};

macro_rules! select {
    ($future:expr, $timeout:expr) => {{
        let future = $future;
        let timeout = $timeout;
        pin_mut!(future);
        pin_mut!(timeout);

        match future::select(future, timeout).await {
            Either::Left((v, _)) => Some(v),
            Either::Right((_, _)) => None,
        }
    }};
}

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

pub struct Dht11<Pin, Delay, HalError>
where
    Pin: InputPin<Error = HalError> + OutputPin<Error = HalError> + Wait,
    Delay: HalDelay,
{
    pin: Pin,
    delay: Delay,
}

/// Represents the current sensor state.
#[derive(Default, Debug)]
pub struct State {
    /// Current temperature.
    pub temperature: ThermodynamicTemperature,
    /// Current humidity.
    pub humidity: Ratio,
}

impl defmt::Format for State {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "State {{ temperature: {}°C, humidity: {}% }}",
            self.temperature.get::<degree_celsius>(),
            self.humidity.value,
        )
    }
}

impl<Pin, Delay, HalError> Dht11<Pin, Delay, HalError>
where
    Pin: InputPin<Error = HalError> + OutputPin<Error = HalError> + Wait,
    Delay: HalDelay,
{
    /// Creates a new [`Dht11`].
    pub fn new(pin: Pin, delay: Delay) -> Self {
        Dht11 { pin, delay }
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
        // Ok(self.read_state().await?)

        Ok(State::default())
    }

    /// Wakes the sensor up.
    async fn wake(&mut self) -> Result<(), HalError> {
        // See: Datasheet § 5.2; figure 2.
        self.pin.set_low()?;
        self.delay.delay_ms(18).await;
        self.pin.set_high()?;
        self.delay.delay_us(40).await;
        Ok(())
    }

    /// Opens a connection to the sensor.
    async fn connect(&mut self) -> Result<(), HalError> {
        // See: datasheet § 5.2-3; figure 3.
        let tolerance = Ratio::new::<ratio>(2.0);
        let timeout = Time::new::<microsecond>(80.0) * tolerance;

        let result = self
            .wait_for_state(PinState::Low, timeout)
            
            // .timed(|x, duration| (x, duration))
            .await;

        // self.wait(PinState::Low, timeout).await?;
        // self.wait(PinState::High, timeout).await?;
        Ok(())
    }

    // async fn read_state(&mut self) -> Result<State, HalError> {
    //     let mut bits: BitArr!(for 40, in u8) = BitArray::ZERO;
    //     for bit in 0..40 {
    //         debug!("reading bit {}", bit);
    //         bits.set(bit, self.read_bit().await?);
    //     }

    //     let expected_checksum = bits[32..].load::<u8>();
    //     let actual_checksum = (bits[..32].iter().fold(0u16, |acc, x| acc + *x as
    // u16) & 0xff) as u8;     if expected_checksum != actual_checksum {
    //         return Err(Error::ChecksumMismatch {
    //             expected: expected_checksum,
    //             actual: actual_checksum,
    //         });
    //     }

    //     Ok(self.parse_state(bits.as_bitslice())?)
    // }

    fn parse_state(&self, bits: &BitSlice<u8>) -> Result<State, HalError> {
        let humidity = bits[0..8].load::<u8>() as f32;
        let temperature = bits[16..24].load::<u8>() as f32;

        if !(0.0..=100.0).contains(&humidity) {
            return Err(Error::InvalidHumidity(humidity));
        }

        if !(0.0..=50.0).contains(&temperature) {
            return Err(Error::InvalidTemperature(temperature));
        }

        Ok(State {
            temperature: ThermodynamicTemperature::new::<degree_celsius>(temperature),
            humidity: Ratio::new::<percent>(humidity),
        })
    }

    // async fn read_bit(&mut self) -> Result<bool, HalError> {
    //     // See: datasheet § 5.3; figure 4.
    //     let tolerance = Ratio::new::<ratio>(1.4);

    //     // Wait for start of transmission.
    //     debug!("waiting for start of transmission...");
    //     if !self.pin.is_low()? {
    //         self.wait(PinState::Low, Time::new::<microsecond>(50.0) * tolerance)
    //             .await?;
    //     }

    //     debug!("reading sensor bit...");
    //     let start_time = Instant::now();
    //     self.wait(PinState::Low, Time::new::<microsecond>(70.0) * tolerance)
    //         .await?;
    //     let end_time = Instant::now();
    //     let bit_duration = end_time - start_time;

    //     // A high level of 26-28μ indicates a `0` bit, 70μ indicates a `1` bit.
    //     if bit_duration.as_micros() < 30 {
    //         Ok(false)
    //     } else {
    //         Ok(true)
    //     }
    // }

    /// Waits for a falling or rising edge until the timeout.
    async fn wait_for_edge(&mut self, timeout: Time) -> Result<PinState, HalError> {
        let timeout = self.delay.delay_us(timeout.get::<microsecond>() as u32);
        match select!(self.pin.wait_for_any_edge(), timeout).transpose()? {
            Some(_) => match self.pin.is_low()? {
                true => Ok(PinState::Low),
                false => Ok(PinState::High),
            },
            None => Err(Error::Timeout),
        }
    }

    /// Waits for a pin state until the timeout. If the pin state is already
    async fn wait_for_state(&mut self, state: PinState, timeout: Time) -> Result<(), HalError> {
        let timeout = self.delay.delay_us(timeout.get::<microsecond>() as u32);
        match state {
            PinState::Low => select!(self.pin.wait_for_low(), timeout).transpose()?,
            PinState::High => select!(self.pin.wait_for_high(), timeout).transpose()?,
        };
        Ok(())
    }
}
