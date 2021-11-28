//! Blinks the LED on a Adafruit Macropad RP2040 board
//!
//! This will blink on-board LED.
//! Also echos USB serial input (minicom -b 115200 -o -D /dev/ttyACM0)
#![no_std]
#![no_main]

mod logger;
mod neopixel;
mod oled_display;
mod panic;

use crate::oled_display::OledDisplay;
use adafruit_macropad::{
    hal::{
        self as rp2040_hal,
        clocks::Clock,
        pac::{self, interrupt},
        pio::PIOExt,
        sio::Sio,
        timer::Timer,
        usb::UsbBus,
        watchdog::Watchdog,
    },
    Pins,
};
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::duration::Extensions;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Hertz;
use log::{info, LevelFilter};
use sh1106::{prelude::*, Builder};
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;
use ws2812_pio::Ws2812;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;

static USB_DEVICE: Mutex<RefCell<Option<UsbDevice<UsbBus>>>> = Mutex::new(RefCell::new(None));

static USB_SERIAL: Mutex<RefCell<Option<SerialPort<UsbBus>>>> = Mutex::new(RefCell::new(None));

static LOGGER: logger::MacropadLogger = logger::MacropadLogger;

static OLED_DISPLAY: Mutex<
    RefCell<
        Option<
            OledDisplay<
                sh1106::interface::SpiInterface<
                    rp2040_hal::spi::Spi<rp2040_hal::spi::Enabled, rp2040_hal::pac::SPI1, 8_u8>,
                    rp2040_hal::gpio::Pin<
                        rp2040_hal::gpio::bank0::Gpio24,
                        rp2040_hal::gpio::Output<rp2040_hal::gpio::PushPull>,
                    >,
                    rp2040_hal::gpio::Pin<
                        rp2040_hal::gpio::bank0::Gpio22,
                        rp2040_hal::gpio::Output<rp2040_hal::gpio::PushPull>,
                    >,
                >,
            >,
        >,
    >,
> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = rp2040_hal::clocks::init_clocks_and_plls(
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

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let mut neopixel = {
        let neopixel_pin = pins.neopixel.into_mode();

        let ws = Ws2812::new(
            neopixel_pin,
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
            timer.count_down(),
        );

        neopixel::Neopixels::new(ws, timer.count_down(), 10.milliseconds())
    };

    let oled_display = {
        // These are implicitly used by the spi driver if they are in the correct mode
        let _spi_sclk = pins.sclk.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let _spi_mosi = pins.mosi.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let _spi_miso = pins.miso.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let spi = rp2040_hal::spi::Spi::<_, _, 8>::new(pac.SPI1);

        // Display control pins
        let oled_dc = pins.oled_dc.into_push_pull_output();
        let oled_cs = pins.oled_cs.into_push_pull_output();
        let mut oled_reset = pins.oled_reset.into_push_pull_output();

        let res = oled_reset.set_high();
        res.unwrap(); //disable screen reset

        // Exchange the uninitialised SPI driver for an initialised one
        let oled_spi = spi.init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            Hertz::new(16_000_000u32),
            &embedded_hal::spi::MODE_0,
        );

        let mut display: GraphicsMode<_> = Builder::new()
            .connect_spi(oled_spi, oled_dc, oled_cs)
            .into();

        display.init().unwrap();
        display.flush().unwrap();

        OledDisplay::new(display)
    };

    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(rp2040_hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    static mut USB_BUS: Option<UsbBusAllocator<rp2040_hal::usb::UsbBus>> = None;

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
        OLED_DISPLAY.borrow(cs).replace(Some(oled_display));
    });

    unsafe {
        // Note (safety): interupts not yet enabled
        log::set_logger_racy(&LOGGER).unwrap();
    }
    log::set_max_level(LevelFilter::Info);

    // Enable the USB interrupt
    unsafe {
        pac::NVIC::unmask(rp2040_hal::pac::Interrupt::USBCTRL_IRQ);
    };

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    //Rust logo splash screen
    cortex_m::interrupt::free(|cs| {
        let mut oled_display_ref = OLED_DISPLAY.borrow(cs).borrow_mut();
        let oled_display = oled_display_ref.as_mut().unwrap();
        oled_display
            .draw_image(include_bytes!("./rust.raw"), 64)
            .unwrap();
    });

    delay.delay_ms(2000);

    info!("macropad starting");

    //Do some example graphics drawing
    cortex_m::interrupt::free(|cs| {
        let mut oled_display_ref = OLED_DISPLAY.borrow(cs).borrow_mut();
        let oled_display = oled_display_ref.as_mut().unwrap();
        oled_display.draw_test().unwrap();
    });

    let led_pin = pins.led.into_push_pull_output();
    let button_pin = pins.button.into_pull_down_input();

    let mut lf = LedFlasher::new(button_pin, led_pin);

    loop {
        //Flash the led
        lf.update().unwrap();

        neopixel.update();
    }
}

fn _show_error<DI, E>(e: UsbError, oled_display: &mut OledDisplay<DI>) -> ()
where
    DI: sh1106::interface::DisplayInterface<Error = E>,
    E: core::fmt::Debug,
{
    match e {
        UsbError::BufferOverflow => oled_display.draw_text_screen("BufferOverflow").unwrap(),
        UsbError::EndpointMemoryOverflow => oled_display
            .draw_text_screen("EndpointMemoryOverflow")
            .unwrap(),
        UsbError::EndpointOverflow => oled_display.draw_text_screen("EndpointOverflow").unwrap(),
        UsbError::InvalidEndpoint => oled_display.draw_text_screen("InvalidEndpoint").unwrap(),
        UsbError::InvalidState => oled_display.draw_text_screen("InvalidState").unwrap(),
        UsbError::ParseError => oled_display.draw_text_screen("ParseError").unwrap(),
        UsbError::Unsupported => oled_display.draw_text_screen("Unsupported").unwrap(),
        UsbError::WouldBlock => {}
    }
}

struct LedFlasher<BP, LP, PinE>
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
    LedFlasher<BP, LP, PinE>
{
    fn new(button_pin: BP, led_pin: LP) -> LedFlasher<BP, LP, PinE>
    where
        BP: InputPin<Error = PinE>,
        LP: OutputPin<Error = PinE>,
        PinE: core::fmt::Debug,
    {
        LedFlasher {
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

            info!("Button pressed {}", self.count);
            self.count = self.count.wrapping_add(1);
        } else if self.button_pin.is_high()? {
            self.led_pin.set_low()?;
            self.pressed = false;
        }

        Ok(())
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| {
        let mut serial_ref = USB_SERIAL.borrow(cs).borrow_mut();
        let serial = serial_ref.as_mut().unwrap();

        // Poll the USB driver with all of our supported USB Classes
        if USB_DEVICE
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .poll(&mut [serial])
        {
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Err(_e) => {
                    // Do nothing
                }
                Ok(_count) => {
                    // Do nothing
                }
            }
        }
    });
}
