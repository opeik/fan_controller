#![no_std]
#![feature(error_in_core)]
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

pub struct Board<'a> {
    pub led: gpio::Output<'a, peripherals::PIN_25>,
    // pub fan_1: Fan<'a, Fan1Control>,
    // pub fan_2: Fan<'a, Fan2Control>,
    // pub fan_3: Fan<'a, Fan3Control>,
    // pub fan_4: Fan<'a, Fan4Control>,
    pub sensor: Mcp9808<'a, Sensor>,
}

impl<'a> Board<'a> {
    pub fn new() -> Result<Self> {
        bind_interrupts!(struct Interrupts {
            I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
        });

        let peripherals = embassy_rp::init(config::Config::default());

        let led = gpio::Output::new(peripherals.PIN_25, Level::Low);
        // let fan_1 = Fan::new(peripherals.PWM_CH0, peripherals.PIN_0);
        // let fan_1 = Fan::new(peripherals.PWM_CH0, peripherals.PIN_0, peripherals.PIN_1);
        // let fan_2 = Fan::new(peripherals.PWM_CH2, peripherals.PIN_4, peripherals.PIN_5);
        // let fan_3 = Fan::new(peripherals.PWM_CH4, peripherals.PIN_8, peripherals.PIN_9);
        // let fan_4 = Fan::new(peripherals.PWM_CH6, peripherals.PIN_12, peripherals.PIN_13);

        let sensor = Mcp9808::new(
            peripherals.I2C0,
            peripherals.PIN_17,
            peripherals.PIN_16,
            Interrupts,
        );

        info!("board initialized!");

        Ok(Self {
            led,
            // fan_1,
            // fan_2,
            // fan_3,
            // fan_4,
            sensor,
        })
    }
}
