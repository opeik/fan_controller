#![no_std]
#![feature(async_fn_in_trait, error_in_core, type_alias_impl_trait)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

use defmt::info;
use driver::{fan::Fan, Mcp9808};
use embassy_rp::{
    bind_interrupts, config,
    gpio::{self, Level},
    i2c, peripherals,
};

type Result<T> = core::result::Result<T, Error>;

/// Represents a board error.
#[derive(Debug, thiserror::Error, defmt::Format)]
pub enum Error {
    /// A fan error occurred.
    #[error("fan error: {0}")]
    FanCurveError(#[from] driver::fan::Error),
}

type Fan1Control = peripherals::PWM_CH0;
type Fan2Control = peripherals::PWM_CH2;
type Fan3Control = peripherals::PWM_CH4;
type Fan4Control = peripherals::PWM_CH6;
type Sensor = peripherals::I2C0;

#[cfg(feature = "wifi")]
use cyw43_pio::PioSpi;

#[cfg(feature = "wifi")]
pub struct Board<'a> {
    pub sensor: Mcp9808<'a, Sensor>,
    pub wifi_spi: PioSpi<'a, peripherals::PIN_25, peripherals::PIO0, 0, peripherals::DMA_CH0>,
    pub wifi_pwr: gpio::Output<'a, peripherals::PIN_23>,
}

#[cfg(not(feature = "wifi"))]
pub struct Board<'a> {
    pub sensor: Mcp9808<'a, Sensor>,
    pub led: gpio::Output<'a, peripherals::PIN_25>,
    // pub fan_1: Fan<'a, Fan1Control>,
}

impl<'a> Board<'a> {
    #[cfg(feature = "wifi")]
    pub fn new() -> Result<Self> {
        use embassy_rp::pio::{self, Pio};

        bind_interrupts!(struct SensorInterrupts {
            I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
        });

        bind_interrupts!(struct WifiInterrupts {
            PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
        });

        let p = embassy_rp::init(config::Config::default());

        // Setup sensors.
        let sensor = Mcp9808::new(p.I2C0, p.PIN_17, p.PIN_16, SensorInterrupts);

        // Setup wifi.
        let pwr = gpio::Output::new(p.PIN_23, Level::Low);
        let cs = gpio::Output::new(p.PIN_25, gpio::Level::High);
        let mut pio = Pio::new(p.PIO0, WifiInterrupts);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            pio.irq0,
            cs,
            p.PIN_24,
            p.PIN_29,
            p.DMA_CH0,
        );

        info!("board initialized!");
        Ok(Self {
            sensor,
            wifi_spi: spi,
            wifi_pwr: pwr,
        })
    }

    #[cfg(not(feature = "wifi"))]
    pub fn new() -> Result<Self> {
        bind_interrupts!(struct Interrupts {
            I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
        });

        let p = embassy_rp::init(config::Config::default());
        let led = gpio::Output::new(p.PIN_25, Level::Low);
        // let fan_1 = Fan::new(peripherals.PWM_CH0, peripherals.PIN_0, peripherals.PIN_1);
        // let fan_2 = Fan::new(peripherals.PWM_CH2, peripherals.PIN_4, peripherals.PIN_5);
        // let fan_3 = Fan::new(peripherals.PWM_CH4, peripherals.PIN_8, peripherals.PIN_9);
        // let fan_4 = Fan::new(peripherals.PWM_CH6, peripherals.PIN_12, peripherals.PIN_13);

        let sensor = Mcp9808::new(p.I2C0, p.PIN_17, p.PIN_16, Interrupts);
        info!("board initialized!");

        Ok(Self { sensor, led })
    }
}
