#![no_std]
#![no_main]
#![feature(async_fn_in_trait, error_in_core, type_alias_impl_trait)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::used_underscore_binding, clippy::missing_errors_doc)]

extern crate alloc;

pub mod driver;
pub mod fan_control;

use defmt::{error, info};
use defmt_rtt as _;
use driver::mcp9808::Mcp9808;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    config::{self},
    gpio::{self, Level},
    i2c, peripherals,
};
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use panic_probe as _;

use crate::{driver::fan::Fan, fan_control::FanControl};

bind_interrupts!(struct Interrupts {
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
});

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    initialize_allocator();
    let peripherals = embassy_rp::init(config::Config::default());
    let mut led = gpio::Output::new(peripherals.PIN_25, Level::Low);
    info!("peripherals initialized!");

    let fan = Fan::builder()
        .control(peripherals.PIN_0, peripherals.PWM_CH0)
        .tachometer(peripherals.PIN_3, peripherals.PWM_CH1)
        .build()
        .unwrap();

    let sensor = Mcp9808::new(
        peripherals.I2C0,
        peripherals.PIN_17,
        peripherals.PIN_16,
        Interrupts,
    );

    let mut fan_control = FanControl::builder()
        .fan(fan)
        .sensor(sensor)
        .build()
        .unwrap();

    loop {
        led.toggle();
        match fan_control.update().await {
            Ok(()) => info!("fan updated"),
            Err(e) => error!("error: {}", e),
        };
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn initialize_allocator() {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }
}
