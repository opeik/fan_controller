#![no_std]
#![no_main]

use board::Board;
use defmt_rtt as _;
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
        board::Board::new().unwrap()
    }

    #[test]
    fn test_manufacturer_id(board: &mut Board<'static>) {
        let manufacturer_id = block_on(board.sensor.manufacturer_id()).unwrap();
        assert_eq!(manufacturer_id, 0x54);
    }
}
