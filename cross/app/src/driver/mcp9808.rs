use embassy_rp::{i2c, interrupt::typelevel::Binding, Peripheral};
pub use fan_controller::decode::mcp9808::Data;
use fan_controller::{
    decode::{
        self,
        mcp9808::raw::{Payload, TemperaturePayload},
    },
    units::ThermodynamicTemperature,
};

type Result<T> = core::result::Result<T, Error>;

/// Represents an  error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("decode error")]
    DecodeError(#[from] decode::mcp9808::Error),
    #[error("i2c error")]
    I2CError(i2c::Error),
}

/// Represents a
#[repr(u8)]
enum Register {
    Temperature = 0x05,
    // ManufacturerId = 0x06,
    // DeviceId = 0x07,
    // Resolution = 0x08,
}

#[derive(derive_builder::Builder)]
#[builder(no_std, pattern = "owned")]
pub struct Mcp9808<'a, T: i2c::Instance> {
    #[builder(setter(custom))]
    i2c: i2c::I2c<'a, T, i2c::Async>,
}

impl<'a, T: i2c::Instance> Mcp9808<'a, T> {
    // const DEVICE_ID: u16 = 0x54;
    const DEFAULT_ADDRESS: u16 = 0x18;

    pub fn new(
        peripheral: impl Peripheral<P = T> + 'a,
        scl_pin: impl Peripheral<P = impl i2c::SclPin<T>> + 'a,
        sda_pin: impl Peripheral<P = impl i2c::SdaPin<T>> + 'a,
        irq: impl Binding<T::Interrupt, i2c::InterruptHandler<T>>,
    ) -> Self {
        Self {
            i2c: i2c::I2c::new_async(peripheral, scl_pin, sda_pin, irq, i2c::Config::default()),
        }
    }

    pub async fn read_temp(&mut self) -> Result<ThermodynamicTemperature> {
        let mut payload = TemperaturePayload::ZERO;

        self.i2c
            .write_async(Self::DEFAULT_ADDRESS, [Register::Temperature as u8])
            .await?;
        self.i2c
            .read_async(Self::DEFAULT_ADDRESS, payload.as_raw_mut_slice())
            .await?;

        match decode::mcp9808::decode(Payload::Temperature(payload))? {
            Data::Temperature(v) => Ok(v),
        }
    }
}

impl<'a, T: i2c::Instance> Mcp9808Builder<'a, T> {
    #[must_use]
    pub fn i2c(
        mut self,
        peripheral: impl Peripheral<P = T> + 'a,
        scl_pin: impl Peripheral<P = impl i2c::SclPin<T>> + 'a,
        sda_pin: impl Peripheral<P = impl i2c::SdaPin<T>> + 'a,
        irq: impl Binding<T::Interrupt, i2c::InterruptHandler<T>>,
    ) -> Self {
        self.i2c = Some(i2c::I2c::new_async(
            peripheral,
            scl_pin,
            sda_pin,
            irq,
            i2c::Config::default(),
        ));
        self
    }
}

impl From<i2c::Error> for Error {
    fn from(value: i2c::Error) -> Self {
        Error::I2CError(value)
    }
}
