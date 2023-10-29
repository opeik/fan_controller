#![no_std]
#![feature(async_fn_in_trait, error_in_core, type_alias_impl_trait)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

use defmt::info;
use driver::{fan::Fan, Mcp9808};
use embassy_rp::{
    bind_interrupts, config,
    gpio::{self, Level, Output},
    i2c,
    peripherals::{self, DMA_CH0, PIN_23, PIN_25, PIO0},
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
    pub wifi_runner: cyw43::Runner<'a, Output<'a, PIN_23>, PioSpi<'a, PIN_25, PIO0, 0, DMA_CH0>>,
    pub wifi_control: cyw43::Control<'a>,
    pub sensor: Mcp9808<'a, Sensor>,
    pub fan_1: Fan<'a, Fan1Control>,
    pub fan_2: Fan<'a, Fan2Control>,
    pub fan_3: Fan<'a, Fan3Control>,
    pub fan_4: Fan<'a, Fan4Control>,
}

#[cfg(not(feature = "wifi"))]
pub struct Board<'a> {
    pub sensor: Mcp9808<'a, Sensor>,
    pub fan_1: Fan<'a, Fan1Control>,
    pub fan_2: Fan<'a, Fan2Control>,
    pub fan_3: Fan<'a, Fan3Control>,
    pub fan_4: Fan<'a, Fan4Control>,
}

#[cfg(not(feature = "wifi"))]
pub struct Led {
    inner: gpio::Output<'a, peripherals::PIN_25>,
}

#[cfg(feature = "wifi")]
pub struct Led {}

impl<'a> Board<'a> {
    #[cfg(feature = "wifi")]
    pub async fn new() -> Result<Self> {
        use embassy_rp::pio::{self, Pio};
        use static_cell::make_static;

        bind_interrupts!(struct SensorInterrupts {
            I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
        });

        bind_interrupts!(struct WifiInterrupts {
            PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
        });

        let p = embassy_rp::init(config::Config::default());

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

        let fw = include_bytes!(env!("RP_PICO_W_FIRMWARE"));
        let clm = include_bytes!(env!("RP_PICO_W_CLM"));
        let state = make_static!(cyw43::State::new());
        let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::PowerSave)
            .await;

        let sensor = Mcp9808::new(p.I2C0, p.PIN_17, p.PIN_16, SensorInterrupts);
        let fan_1 = Fan::new(p.PWM_CH0, p.PIN_0, p.PIN_1);
        let fan_2 = Fan::new(p.PWM_CH2, p.PIN_4, p.PIN_5);
        let fan_3 = Fan::new(p.PWM_CH4, p.PIN_8, p.PIN_9);
        let fan_4 = Fan::new(p.PWM_CH6, p.PIN_12, p.PIN_13);
        info!("board initialized!");

        Ok(Self {
            wifi_runner: runner,
            wifi_control: control,
            sensor,
            fan_1,
            fan_2,
            fan_3,
            fan_4,
        })
    }

    #[cfg(not(feature = "wifi"))]
    pub fn new() -> Result<Self> {
        bind_interrupts!(struct Interrupts {
            I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
        });

        let p = embassy_rp::init(config::Config::default());
        let led = gpio::Output::new(p.PIN_25, Level::Low);
        let fan_1 = Fan::new(p.PWM_CH0, p.PIN_0, p.PIN_1);
        let fan_2 = Fan::new(p.PWM_CH2, p.PIN_4, p.PIN_5);
        let fan_3 = Fan::new(p.PWM_CH4, p.PIN_8, p.PIN_9);
        let fan_4 = Fan::new(p.PWM_CH6, p.PIN_12, p.PIN_13);

        let sensor = Mcp9808::new(p.I2C0, p.PIN_17, p.PIN_16, Interrupts);
        info!("board initialized!");

        Ok(Self {
            sensor,
            led,
            fan_1,
            fan_2,
            fan_3,
            fan_4,
        })
    }
}
