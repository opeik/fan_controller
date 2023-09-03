use core::convert::Infallible;

use embassy_rp::gpio::{Flex, Pin};
use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use embedded_hal_async::digital::Wait;

pub struct InputOutputPin<'d, T: Pin> {
    pub pin: Flex<'d, T>,
}

impl<'d, T: Pin> ErrorType for InputOutputPin<'d, T> {
    type Error = Infallible;
}
impl<'d, T: Pin> InputPin for InputOutputPin<'d, T> {
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.pin.is_high())
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(self.pin.is_low())
    }
}

impl<'d, T: Pin> OutputPin for InputOutputPin<'d, T> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.pin.set_low();
        self.pin.set_as_output();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.pin.set_high();
        self.pin.set_as_output();
        Ok(())
    }
}

impl<'d, T: Pin> Wait for InputOutputPin<'d, T> {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        self.pin.set_as_input();
        self.pin.wait_for_high().await;
        Ok(())
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        self.pin.set_as_input();
        self.pin.wait_for_low().await;
        Ok(())
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        self.pin.set_as_input();
        self.pin.wait_for_rising_edge().await;
        Ok(())
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        self.pin.set_as_input();
        self.pin.wait_for_falling_edge().await;
        Ok(())
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        self.pin.set_as_input();
        self.pin.wait_for_any_edge().await;
        Ok(())
    }
}
