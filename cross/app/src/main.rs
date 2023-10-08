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
use embassy_time::{Duration, Timer};
use embedded_alloc::Heap;
use panic_probe as _;

use crate::fan_control::FanControl;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    initialize_allocator();
    let mut board = Board::new().expect("failed to initialize board");

    // let mut fan_control = FanControl::builder()
    //     .fan(fan)
    //     .sensor(sensor)
    //     .build()
    //     .unwrap();

    loop {
        board.led.toggle();
        // match fan_control.update().await {
        //     Ok(()) => info!("fan updated"),
        //     Err(e) => error!("error: {}", e),
        // };
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
