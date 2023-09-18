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
//! See: [datasheet] § 5.
//!
//! [datasheet]: https://www.mouser.com/datasheet/2/758/DHT11-Technical-Data-Sheet-Translated-Version-1143054.pdf
use bitvec::prelude::*;
use defmt::{debug, trace};
use embassy_rp::gpio;
use embassy_time::{Duration, Timer};
use embedded_hal::digital::PinState;
use fan_controller::decode::{self, dht::Data};

use crate::future::timed;

type Result<T> = core::result::Result<T, Error>;

/// Represents a sensor error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    /// The sensor is not present.
    #[error("sensor not present")]
    NotPresent,
    /// The read timed out.
    #[error("read timed out")]
    Timeout,
    /// A bit's high voltage level duration too long.
    #[error("found suspect bit")]
    SuspectBit,
    /// A decode error occurred.
    #[error("decode error: {0}")]
    DecodeError(decode::dht::Error),
}

/// Represents a DHT11 temperature and humidity sensor.
///
/// See: [the datasheet].
///
/// [the datasheet]: https://www.mouser.com/datasheet/2/758/DHT11-Technical-Data-Sheet-Translated-Version-1143054.pdf
pub struct Dht11<'a, Pin>
where
    Pin: gpio::Pin,
{
    pin: gpio::OutputOpenDrain<'a, Pin>,
}

/// Represents raw [`Dht11`] sensor data.
#[derive(Default, Debug, PartialEq)]
struct RawData {
    pub humidity: u8,
    pub humidity_frac: u8,
    pub temperature: u8,
    pub temperature_frac: u8,
}

impl<'a, Pin> Dht11<'a, Pin>
where
    Pin: gpio::Pin,
{
    /// Creates a new [`Dht11`].
    pub fn new(pin: gpio::OutputOpenDrain<'a, Pin>) -> Self {
        Dht11 { pin }
    }

    /// Reads data from the sensor.
    pub async fn read(&mut self) -> Result<Data> {
        debug!("connecting to dht11...");
        self.connect().await?;
        debug!("reading from dht11...");
        self.read_data().await
    }

    /// Connects to the sensor.
    async fn connect(&mut self) -> Result<()> {
        const TOLERANCE_US: u64 = 10;

        // See: Datasheet § 5.2; figure 2.
        self.pin.set_low();
        Timer::after(Duration::from_micros(30)).await;
        self.pin.set_high();
        Timer::after(Duration::from_micros(40)).await;

        // See: datasheet § 5.2-3; figure 3.
        let timeout_us = 80 + TOLERANCE_US; // 10μs tolerance.
        self.wait_for(PinState::High, timeout_us)
            .await
            .map_err(|_| Error::NotPresent)?;
        self.wait_for(PinState::Low, timeout_us)
            .await
            .map_err(|_| Error::NotPresent)?;

        Ok(())
    }

    /// Implements reading data from the sensor.
    async fn read_data(&mut self) -> Result<Data> {
        let mut data = bitarr![u8, Msb0; 0; 40];
        for mut bit in &mut data {
            *bit = self.read_bit().await?;
        }

        trace!("read data: {:08b}", data.as_raw_slice());
        decode::dht::decode(data.as_bitslice()).map_err(Error::DecodeError)
    }

    /// Reads a bit of data from the sensor.
    async fn read_bit(&mut self) -> Result<bool> {
        const TOLERANCE_US: u64 = 10;

        // See: datasheet § 5.3; figure 4.
        self.wait_for(PinState::High, 50).await?;
        let (result, duration) = timed!(self.wait_for(PinState::Low, 70 + TOLERANCE_US));
        result?;

        // A high level of ~30μ indicates a `0` bit, 70μ indicates a `1` bit.
        match duration.as_micros() {
            0..=40 => Ok(false),
            41..=80 => Ok(true),
            _ => Err(Error::SuspectBit),
        }
    }

    /// Waits for a pin state until the timeout.
    async fn wait_for(&mut self, state: PinState, timeout_us: u64) -> Result<()> {
        let timeout = Duration::from_micros(timeout_us);
        let result = match state {
            PinState::Low => embassy_time::with_timeout(timeout, self.pin.wait_for_low()).await,
            PinState::High => embassy_time::with_timeout(timeout, self.pin.wait_for_high()).await,
        };

        match result {
            Ok(()) => Ok(()),
            Err(_) => Err(Error::Timeout),
        }
    }
}
