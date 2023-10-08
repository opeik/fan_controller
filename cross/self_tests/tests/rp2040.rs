#![no_std]
#![no_main]
#![feature(error_in_core)]

use board::Board;
use defmt_rtt as _;
use driver::mcp9808::{self, Revision};
use embassy_futures::block_on;
use embedded_alloc::Heap;
use panic_probe as _;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[defmt_test::tests]
mod tests {
    use super::*;

    #[init]
    fn init() -> Board<'static> {
        board::Board::new().expect("failed to initialize board")
    }

    #[test]
    fn test_manufacturer_id(board: &mut Board<'static>) {
        let manufacturer_id = block_on(board.sensor.manufacturer_id()).unwrap();
        assert_eq!(manufacturer_id, mcp9808::MANUFACTURER_ID);
    }

    #[test]
    fn test_device_id(board: &mut Board<'static>) {
        let (device_id, revision) = block_on(board.sensor.device_id()).unwrap();
        assert_eq!(device_id, mcp9808::DEVICE_ID);
        assert_eq!(revision, Revision(0x00));
    }
}
