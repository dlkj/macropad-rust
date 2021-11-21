//! Blinks the LED on a Adafruit Macropad RP2040 board
//!
//! This will blink on-board LED.
//! Also echos USB serial input (minicom -b 115200 -o -D /dev/ttyACM0)
#![no_std]
#![no_main]

mod oled_display;

use crate::oled_display::OledDisplay;

use adafruit_macropad::{
    hal,
    hal::{
        clocks::Clock,
        pac::{self, interrupt},
        sio::Sio,
        watchdog::Watchdog,
    },
    Pins,
};
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::units::Extensions;
use panic_halt as _;
use sh1106::{prelude::*, Builder};
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;

static USB_DEVICE: Mutex<RefCell<Option<UsbDevice<hal::usb::UsbBus>>>> =
    Mutex::new(RefCell::new(None));

static USB_SERIAL: Mutex<RefCell<Option<SerialPort<hal::usb::UsbBus>>>> =
    Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = hal::clocks::init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // These are implicitly used by the spi driver if they are in the correct mode
    let _spi_sclk = pins.sclk.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_mosi = pins.mosi.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_miso = pins.miso.into_mode::<hal::gpio::FunctionSpi>();
    let spi = hal::spi::Spi::<_, _, 8>::new(pac.SPI1);

    // Display control pins
    let oled_dc = pins.oled_dc.into_push_pull_output();
    let oled_cs = pins.oled_cs.into_push_pull_output();
    let mut oled_reset = pins.oled_reset.into_push_pull_output();
    oled_reset.set_high().unwrap(); //disable screen reset

    // Exchange the uninitialised SPI driver for an initialised one
    let oled_spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_000_000u32.Hz(),
        &embedded_hal::spi::MODE_0,
    );

    let mut display: GraphicsMode<_> = Builder::new()
        .connect_spi(oled_spi, oled_dc, oled_cs)
        .into();

    display.init().unwrap();
    display.flush().unwrap();

    let mut oled_display = OledDisplay::new(display);

    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

    unsafe {
        // Note (safety): This is safe as interrupts haven't been started yet
        USB_BUS = Some(usb_bus);
    }

    // Grab a reference to the USB Bus allocator. We are promising to the
    // compiler not to take mutable access to this global variable whilst this
    // reference exists!
    let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

    // Set up the USB Communications Class Device driver
    let serial = SerialPort::new(bus_ref);

    // Create a USB device with a fake VID and PID
    let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    cortex_m::interrupt::free(|cs| {
        USB_SERIAL.borrow(cs).replace(Some(serial));
        USB_DEVICE.borrow(cs).replace(Some(usb_dev));
    });

    // Enable the USB interrupt
    unsafe {
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
    };

    //USB code now running

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    oled_display
        .draw_image(include_bytes!("./rust.raw"), 64)
        .unwrap();

    delay.delay_ms(500);

    oled_display.draw_test().unwrap();

    let led_pin: adafruit_macropad::hal::gpio::Pin<_, _> = pins.led.into_push_pull_output();
    let button_pin: adafruit_macropad::hal::gpio::Pin<_, _> = pins.button.into_pull_down_input();
    let mut s = State::new(button_pin, led_pin);

    loop {
        s.update().unwrap();
    }
}

struct State<BP, LP, PinE>
where
    BP: InputPin<Error = PinE>,
    LP: OutputPin<Error = PinE>,
    PinE: core::fmt::Debug,
{
    count: u8,
    pressed: bool,
    button_pin: BP,
    led_pin: LP,
}

impl<BP: InputPin<Error = PinE>, LP: OutputPin<Error = PinE>, PinE: core::fmt::Debug>
    State<BP, LP, PinE>
{
    fn new(button_pin: BP, led_pin: LP) -> State<BP, LP, PinE>
    where
        BP: InputPin<Error = PinE>,
        LP: OutputPin<Error = PinE>,
        PinE: core::fmt::Debug,
    {
        State {
            count: 0,
            pressed: false,
            button_pin,
            led_pin,
        }
    }

    fn update(&mut self) -> Result<(), PinE> {
        if self.button_pin.is_low()? && !self.pressed {
            self.led_pin.set_high()?;
            self.pressed = true;

            // We do this with interrupts disabled, to avoid a race hazard with the USB IRQ.
            cortex_m::interrupt::free(|cs| {
                // Now interrupts are disabled, grab the global variable and, if
                // available, send it a HID report
                serial_write(cs, b"Hello, World! ").unwrap();

                let mut count_str = [0u8, 64];
                count_str[0] = (self.count % 10) + 48; //generate asci digits
                count_str[1] = 0;
                serial_write(cs, &count_str).unwrap();
                serial_write(cs, b"\r\n").unwrap()
            });

            self.count = (self.count + 1) % 10;
        } else if self.button_pin.is_high()? {
            self.led_pin.set_low()?;
            self.pressed = false;
        }

        Ok(())
    }
}

/// This function is called whenever the USB Hardware generates an Interrupt
/// Request.
///
/// We do all our USB work under interrupt, so the main thread can continue on
/// knowing nothing about USB.
#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    use core::sync::atomic::{AtomicBool, Ordering};

    /// Note whether we've already printed the "hello" message.
    static SAID_HELLO: AtomicBool = AtomicBool::new(false);

    cortex_m::interrupt::free(|cs| {
        // Say hello exactly once on start-up
        if !SAID_HELLO.load(Ordering::Relaxed) {
            SAID_HELLO.store(true, Ordering::Relaxed);
            let _ = serial_write(cs, b"Hello, World!\r\n");
        }
        // Poll the USB driver with all of our supported USB Classes
        if USB_DEVICE
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .poll(&mut [USB_SERIAL.borrow(cs).borrow_mut().as_mut().unwrap()])
        {
            let mut buf = [0u8; 64];
            match serial_read(cs, &mut buf) {
                Err(_e) => {
                    // Do nothing
                }
                Ok(0) => {
                    // Do nothing
                }
                Ok(count) => {
                    // Convert to upper case
                    buf.iter_mut().take(count).for_each(|b| {
                        b.make_ascii_uppercase();
                    });
                    // Send back to the host
                    let mut wr_ptr = &buf[..count];
                    while !wr_ptr.is_empty() {
                        let _ = serial_write(cs, wr_ptr).map(|len| {
                            wr_ptr = &wr_ptr[len..];
                        });
                    }
                }
            }
        }
    });
}

fn serial_write(cs: &cortex_m::interrupt::CriticalSection, data: &[u8]) -> Result<usize, UsbError> {
    USB_SERIAL
        .borrow(cs)
        .borrow_mut()
        .as_mut()
        .unwrap()
        .write(data)
}

fn serial_read(
    cs: &cortex_m::interrupt::CriticalSection,
    data: &mut [u8],
) -> Result<usize, UsbError> {
    USB_SERIAL
        .borrow(cs)
        .borrow_mut()
        .as_mut()
        .unwrap()
        .read(data)
}
