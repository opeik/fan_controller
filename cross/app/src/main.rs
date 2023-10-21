#![no_std]
#![no_main]
#![feature(async_fn_in_trait, error_in_core, type_alias_impl_trait)]
#![warn(clippy::suspicious, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(clippy::used_underscore_binding, clippy::missing_errors_doc)]

extern crate alloc;

pub mod fan_control;

use board::Board;
use defmt::{error, info};
use defmt_rtt as _;
use driver::{Fan, Mcp9808};
use embassy_executor::Spawner;
use embassy_rp::{gpio, peripherals, pio, pio::Pio};
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use panic_probe as _;
use static_cell::make_static;

use crate::fan_control::FanControl;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[cfg(feature = "wifi")]
#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        gpio::Output<'static, peripherals::PIN_23>,
        cyw43_pio::PioSpi<'static, peripherals::PIN_25, peripherals::PIO0, 0, peripherals::DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_allocator();
    let mut board = Board::new().expect("failed to initialize board");

    #[cfg(feature = "wifi")]
    {
        use defmt::unwrap;
        let fw = include_bytes!(env!("RP_PICO_W_FIRMWARE"));
        let clm = include_bytes!(env!("RP_PICO_W_CLM"));
        let state = make_static!(cyw43::State::new());
        let (_net_device, mut control, runner) =
            cyw43::new(state, board.wifi_pwr, board.wifi_spi, fw).await;
        unwrap!(spawner.spawn(wifi_task(runner)));

        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::PowerSave)
            .await;
    }

    // let mut fan_1_control = FanControl::builder()
    //     .fan(board.fan_1)
    //     .sensor(board.sensor)
    //     .build()
    //     .unwrap();

    loop {
        // board.led.toggle();
        info!("hi");
        // match fan_1_control.update().await {
        //     Ok(()) => info!("fan updated"),
        //     Err(e) => error!("error: {}", e),
        // };
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn init_allocator() {
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }
}
