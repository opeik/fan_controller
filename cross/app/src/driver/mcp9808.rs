const DEVICE_ID: u16 = 0x54;
const DEFAULT_ADDRESS: u16 = 0x18;

#[repr(u8)]
enum Register {
    Configuration = 0x01,
    UpperTemperature = 0x02,
    LowerTemperature = 0x03,
    CriticalTemperature = 0x04,
    Temperature = 0x05,
    ManufacturerId = 0x06,
    DeviceId = 0x07,
    Resolution = 0x08,
}
