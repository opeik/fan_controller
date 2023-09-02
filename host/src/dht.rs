use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_async::{delay::DelayUs as HalDelay, digital::Wait};
use futures::{
    future::{self, Either},
    pin_mut,
};
use uom::si::f32::{Ratio, ThermodynamicTemperature};

type Result<T, E> = core::result::Result<T, Error<E>>;

/// Represents a sensor error.
#[derive(Debug, thiserror::Error)]
pub enum Error<HalError> {
    /// The connection was refused by the DHT sensor.
    #[error("failed to connect to the DHT sensor")]
    ConnectionRefused,
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
pub struct State {
    /// Current temperature.
    pub temperature: ThermodynamicTemperature,
    /// Current humidity.
    pub humidity: Ratio,
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
        self.wake().await?;
        self.connect().await?;

        todo!()
    }

    /// Wakes the sensor up.
    async fn wake(&mut self) -> Result<(), HalError> {
        // See: datasheet § 5.2.
        self.pin.set_low()?;
        self.delay.delay_ms(18).await;
        self.pin.set_high()?;
        self.delay.delay_us(40).await;
        Ok(())
    }

    /// Opens a connection to the sensor.
    async fn connect(&mut self) -> Result<(), HalError> {
        // See: datasheet § 5.2-3.
        let falling_edge = self.pin.wait_for_falling_edge();
        let timeout = self.delay.delay_us(80);
        pin_mut!(falling_edge);
        pin_mut!(timeout);

        match future::select(falling_edge, timeout).await {
            Either::Left((_, _)) => {}
            Either::Right((_, _)) => return Err(Error::<HalError>::ConnectionRefused),
        };

        Ok(())
    }
}
