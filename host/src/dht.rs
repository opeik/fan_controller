use defmt::debug;
use embedded_hal::digital::{InputPin, OutputPin, PinState};
use embedded_hal_async::{delay::DelayUs as HalDelay, digital::Wait};
use futures::{
    future::{self, Either},
    pin_mut,
};
use uom::si::{
    f32::{Ratio, ThermodynamicTemperature, Time},
    ratio::percent,
    thermodynamic_temperature::degree_celsius,
    time::nanosecond,
};

macro_rules! select_timeout {
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
            self.humidity.value * 100.0,
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
        Ok(self.read_state().await?)
    }

    /// Wakes the sensor up.
    async fn wake(&mut self) -> Result<(), HalError> {
        // See: Datasheet § 5.2.
        self.pin.set_low()?;
        self.delay.delay_ms(18).await;
        self.pin.set_high()?;
        self.delay.delay_us(40).await;

        Ok(())
    }

    /// Opens a connection to the sensor.
    async fn connect(&mut self) -> Result<(), HalError> {
        // The datasheet claims pin is pulled low for 80μ. In my testing it never
        // hit that deadline.
        const CRINGE_FACTOR: f32 = 2.5;

        // See: datasheet § 5.2-3.
        let timeout = Time::new::<nanosecond>(80.0 * CRINGE_FACTOR);
        self.wait_for_pin_state(PinState::Low, timeout).await?;
        self.wait_for_pin_state(PinState::High, timeout).await?;

        Ok(())
    }

    async fn read_state(&mut self) -> Result<State, HalError> {
        Ok(State::default())
    }

    async fn wait_for_pin_state(
        &mut self,
        pin_state: PinState,
        timeout: Time,
    ) -> Result<Option<()>, HalError> {
        let timeout = self.delay.delay_us(timeout.get::<nanosecond>() as u32);
        let result = match pin_state {
            PinState::Low => select_timeout!(self.pin.wait_for_falling_edge(), timeout),
            PinState::High => select_timeout!(self.pin.wait_for_rising_edge(), timeout),
        };
        Ok(result.transpose()?)
    }
}
