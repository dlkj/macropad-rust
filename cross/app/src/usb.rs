use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_hid::descriptor::KeyboardReport;
use usbd_hid::descriptor::SerializedDescriptor;
use usbd_hid::hid_class::HIDClass;
use usbd_serial::SerialPort;

pub struct UsbManager<'a, B>
where
    B: usb_device::bus::UsbBus,
{
    usb_device: UsbDevice<'a, B>,
    serial_port: SerialPort<'a, B>,
    keyboard: HIDClass<'a, B>,
}

impl<'a, B> UsbManager<'a, B>
where
    B: usb_device::bus::UsbBus,
{
    pub fn new(usb_bus: &'a UsbBusAllocator<B>) -> UsbManager<'a, B> {
        let serial_port = SerialPort::new(usb_bus);
        let keyboard = HIDClass::new(usb_bus, KeyboardReport::desc(), 20);

        // Create a USB device with a fake VID and PID
        let usb_device = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Adafruit")
            .product("Macropad")
            .serial_number("TEST")
            .device_class(0x00) // from: https://www.usb.org/defined-class-codes
            .build();

        UsbManager {
            serial_port,
            keyboard,
            usb_device,
        }
    }

    pub fn keyboard_borrow_mut(&mut self) -> &mut HIDClass<'a, B> {
        &mut self.keyboard
    }

    pub fn serial_port_borrow_mut(&mut self) -> &mut SerialPort<'a, B> {
        &mut self.serial_port
    }

    pub fn service_irq(&mut self) {
        // Poll the USB driver with all of our supported USB Classes
        if self
            .usb_device
            .poll(&mut [&mut self.serial_port, &mut self.keyboard])
        {
            let mut buf = [0u8; 64];
            match self.serial_port.read(&mut buf) {
                Err(_e) => {}
                Ok(_count) => {}
            }

            let mut buf = [0u8; 64];
            match self.keyboard.pull_raw_output(&mut buf) {
                Err(_e) => {}
                Ok(_count) => {}
            }
        }
    }
}
