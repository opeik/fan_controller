use embedded_hal::digital::{InputPin, OutputPin};

pub struct Fan<ControlPin, TachometerPin, HalError>
where
    ControlPin: OutputPin<Error = HalError>,
    TachometerPin: InputPin<Error = HalError>,
{
    control_pin: ControlPin,
    tachometer_pin: TachometerPin,
}

impl<ControlPin, TachometerPin, HalError> Fan<ControlPin, TachometerPin, HalError>
where
    ControlPin: OutputPin<Error = HalError>,
    TachometerPin: InputPin<Error = HalError>,
{
    pub fn new(control_pin: ControlPin, tachometer_pin: TachometerPin) -> Self {
        Self {
            control_pin,
            tachometer_pin,
        }
    }
}
