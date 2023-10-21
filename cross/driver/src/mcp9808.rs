use embassy_rp::{i2c, interrupt::typelevel::Binding, Peripheral};
pub use fan_controller::decode::mcp9808::{
    Address, DeviceId, ManufacturerId, ManufacturerIdPayload, Revision, TemperaturePayload,
};
use fan_controller::{
    decode::{self, mcp9808::DeviceIdPayload},
    units::ThermodynamicTemperature,
};

type Result<T> = core::result::Result<T, Error>;

/// Represents an error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    #[error("decode error")]
    DecodeError(#[from] decode::mcp9808::Error),
    #[error("i2c error")]
    I2CError(i2c::Error),
}

/// Represents a MCP9808 register.
///
/// See: datasheet § 5.1, page 16.
#[repr(u8)]
enum Register {
    Temperature = 0x05,
    ManufacturerId = 0x06,
    DeviceId = 0x07,
}

#[derive(derive_builder::Builder)]
#[builder(no_std, pattern = "owned")]
pub struct Mcp9808<'a, T: i2c::Instance> {
    #[builder(setter(custom))]
    i2c: i2c::I2c<'a, T, i2c::Async>,
}

/// Standard MCP9808 manufacturer ID.
pub const MANUFACTURER_ID: ManufacturerId = ManufacturerId(0x54);
/// Standard MCP9808 device ID.
pub const DEVICE_ID: DeviceId = DeviceId(0x04);
/// Default MCP9808 I2C address.
pub const DEFAULT_ADDRESS: Address = Address(0x18);

impl<'a, T: i2c::Instance> Mcp9808<'a, T> {
    #[must_use]
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

    pub async fn temp(&mut self) -> Result<ThermodynamicTemperature> {
        let mut payload = TemperaturePayload::ZERO;

        self.i2c
            .write_async(*DEFAULT_ADDRESS, [Register::Temperature as u8])
            .await?;
        self.i2c
            .read_async(*DEFAULT_ADDRESS, payload.as_raw_mut_slice())
            .await?;

        Ok(decode::mcp9808::decode_temperature(payload)?)
    }

    pub async fn manufacturer_id(&mut self) -> Result<ManufacturerId> {
        let mut payload = ManufacturerIdPayload::ZERO;

        self.i2c
            .write_async(*DEFAULT_ADDRESS, [Register::ManufacturerId as u8])
            .await?;
        self.i2c
            .read_async(*DEFAULT_ADDRESS, payload.as_raw_mut_slice())
            .await?;

        Ok(decode::mcp9808::decode_manufacturer_id(payload)?)
    }

    pub async fn device_id(&mut self) -> Result<(DeviceId, Revision)> {
        let mut payload = DeviceIdPayload::ZERO;

        self.i2c
            .write_async(*DEFAULT_ADDRESS, [Register::DeviceId as u8])
            .await?;
        self.i2c
            .read_async(*DEFAULT_ADDRESS, payload.as_raw_mut_slice())
            .await?;

        Ok(decode::mcp9808::decode_device_id(payload)?)
    }
}

impl From<i2c::Error> for Error {
    fn from(value: i2c::Error) -> Self {
        Error::I2CError(value)
    }
}
