#![no_std]
#![no_main]

use {defmt_rtt as _, panic_probe as _};

#[defmt_test::tests]
mod tests {
    use defmt::assert_eq;
    use embassy_rp::Peripherals;

    #[init]
    fn init() -> Peripherals {
        embassy_rp::init(Default::default())
    }

    #[test]
    fn hello(_board: &mut Peripherals) {
        assert_eq!(1, 1)
    }
}
