#![no_std]
#![no_main]

//! Blinks the LED on a Adafruit Macropad RP2040 board
//!
//! This will blink on-board LED.
//! Also echos USB serial input (minicom -b 115200 -o -D /dev/ttyACM0)

mod logger;
mod macropad;
mod neopixel;
mod oled_display;
mod panic;

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
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::prelude::*;
use embedded_time::duration::Extensions;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Hertz;
use log::{info, LevelFilter};
use rp2040_hal::gpio::dynpin::DynPin;
use sh1106::{prelude::*, Builder};
use usb_device::{class_prelude::*, prelude::*};
use usbd_hid::descriptor::KeyboardReport;
use usbd_hid::descriptor::SerializedDescriptor;
use usbd_hid::hid_class::HIDClass;
use usbd_serial::SerialPort;
use ws2812_pio::Ws2812;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;

type Spi = rp2040_hal::spi::Spi<rp2040_hal::spi::Enabled, rp2040_hal::pac::SPI1, 8_u8>;
type OledDisplay = oled_display::OledDisplay<sh1106::interface::SpiInterface<Spi, DynPin, DynPin>>;

static USB_DEVICE: Mutex<RefCell<Option<UsbDevice<UsbBus>>>> = Mutex::new(RefCell::new(None));

static USB_SERIAL: Mutex<RefCell<Option<SerialPort<UsbBus>>>> = Mutex::new(RefCell::new(None));
static USB_KEYBOARD: Mutex<RefCell<Option<HIDClass<UsbBus>>>> = Mutex::new(RefCell::new(None));

static LOGGER: logger::MacropadLogger = logger::MacropadLogger;

static OLED_DISPLAY: Mutex<RefCell<Option<OledDisplay>>> = Mutex::new(RefCell::new(None));

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

    //init neopixels
    let mut neopixel = {
        let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
        let neopixel_pin = pins.neopixel.into_mode();

        let ws = Ws2812::new(
            neopixel_pin,
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
            timer.count_down(),
        );

        neopixel::Neopixels::new(ws)
    };

    //init the oled display
    {
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
            .connect_spi(oled_spi, oled_dc.into(), oled_cs.into())
            .into();

        display.init().unwrap();
        display.flush().unwrap();

        cortex_m::interrupt::free(|cs| {
            OLED_DISPLAY
                .borrow(cs)
                .replace(Some(OledDisplay::new(display)));
        });
    }

    //Init USB
    static mut USB_BUS: Option<UsbBusAllocator<rp2040_hal::usb::UsbBus>> = None;

    {
        // Set up the USB driver
        let usb_bus = UsbBusAllocator::new(rp2040_hal::usb::UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        ));

        cortex_m::interrupt::free(|_cs| unsafe {
            // Note (safety): This is safe as interrupts are masked
            USB_BUS = Some(usb_bus);
        });
    }

    cortex_m::interrupt::free(|cs| {
        // Note (safety): This is safe as interrupts are masked
        let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

        // Set up the USB Communications Class Device driver
        USB_SERIAL
            .borrow(cs)
            .replace(Some(SerialPort::new(bus_ref)));

        // Set up the USB HID Device drivers
        USB_KEYBOARD
            .borrow(cs)
            .replace(Some(HIDClass::new(bus_ref, KeyboardReport::desc(), 50)));

        // Create a USB device with a fake VID and PID
        let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Adafruit")
            .product("Macropad")
            .serial_number("TEST")
            .device_class(0x00) // from: https://www.usb.org/defined-class-codes
            .build();

        USB_DEVICE.borrow(cs).replace(Some(usb_dev));

        unsafe {
            // Note (safety): interupts not yet enabled
            log::set_logger_racy(&LOGGER).unwrap();
        }
    });

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

    let keys: [DynPin; 12] = [
        pins.key1.into_pull_up_input().into(),
        pins.key2.into_pull_up_input().into(),
        pins.key3.into_pull_up_input().into(),
        pins.key4.into_pull_up_input().into(),
        pins.key5.into_pull_up_input().into(),
        pins.key6.into_pull_up_input().into(),
        pins.key7.into_pull_up_input().into(),
        pins.key8.into_pull_up_input().into(),
        pins.key9.into_pull_up_input().into(),
        pins.key10.into_pull_up_input().into(),
        pins.key11.into_pull_up_input().into(),
        pins.key12.into_pull_up_input().into(),
    ];

    //keypad, final row: '0', '.', 'enter'
    const KEY_MAP: [u8; 12] = [
        0x5f, 0x60, 0x61, 0x5c, 0x5d, 0x5e, 0x59, 0x5a, 0x5b, 0x62, 0x63, 0x58,
    ];
    let mut mp = macropad::Macropad::new(keys, KEY_MAP);

    let mut fast_countdown = timer.count_down();
    fast_countdown.start(1.milliseconds());

    let mut slow_countdown = timer.count_down();
    slow_countdown.start(10.milliseconds());

    loop {
        //1ms scan the keys and debounce
        if fast_countdown.wait().is_ok() {
            mp.update().expect("Failed to update macro pad");
        }

        //10ms
        if slow_countdown.wait().is_ok() {
            //get first 6 current keypresses and send to usb
            let mut keycodes: [u8; 6] = [0, 0, 0, 0, 0, 0];
            for (i, code) in mp.get_keycodes().iter().take(keycodes.len()).enumerate() {
                keycodes[i] = *code;
            }

            cortex_m::interrupt::free(|cs| {
                let mut keyboard_ref = USB_KEYBOARD.borrow(cs).borrow_mut();
                if let Some(keyboard) = keyboard_ref.as_mut() {
                    let _ = keyboard.push_input(&KeyboardReport {
                        modifier: 0,
                        leds: 0,
                        reserved: 0,
                        keycodes,
                    });
                }
            });

            //update the screen
            cortex_m::interrupt::free(|cs| {
                let mut oled_display_ref = OLED_DISPLAY.borrow(cs).borrow_mut();
                if let Some(oled_display) = oled_display_ref.as_mut() {
                    oled_display.draw_numpad().unwrap();
                }
            });

            //update the LEDs
            neopixel.update().unwrap();
        }
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| {
        let mut serial_ref = USB_SERIAL.borrow(cs).borrow_mut();
        let serial = serial_ref.as_mut().unwrap();

        let mut keyboard_ref = USB_KEYBOARD.borrow(cs).borrow_mut();
        let keyboard = keyboard_ref.as_mut().unwrap();

        // Poll the USB driver with all of our supported USB Classes
        if USB_DEVICE
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .poll(&mut [serial, keyboard])
        {
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Err(_e) => {}
                Ok(_count) => {}
            }

            let mut buf = [0u8; 64];
            match keyboard.pull_raw_output(&mut buf) {
                Err(_e) => {}
                Ok(_count) => {}
            }
        }
    });
}
