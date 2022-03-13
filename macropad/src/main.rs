#![no_std]
#![no_main]

use core::cell::{Cell, RefCell};
use core::convert::Infallible;
use core::sync::atomic::Ordering;

use adafruit_macropad::{
    hal::{
        self as hal,
        clocks::Clock,
        pac::{self, interrupt},
    },
    Pins,
};
use atomic_polyfill::AtomicU8;
use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
use embedded_hal::digital::v2::{InputPin, OutputPin, ToggleableOutputPin};
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Hertz;
use frunk::HList;
use log::LevelFilter;
use panic_persist as _;
use sh1106::prelude::*;
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_hid_devices::device::consumer::ConsumerControlInterface;
use usbd_hid_devices::device::keyboard::NKROBootKeyboardInterface;
use usbd_hid_devices::device::mouse::WheelMouseInterface;
use usbd_hid_devices::prelude::*;

use crate::debounce::DebouncedInputPin;
use crate::debounced_input_array::DebouncedInputArray;
use crate::display_controller::DisplayController;
use crate::keypad_controller::KeypadController;
use crate::logger::Logger;
use crate::models::{ApplicationModel, DisplayModel, PeripheralsModel, UsbModel};
use crate::time::TimerClock;

mod debounce;
mod debounced_input_array;
mod display_controller;
mod keypad_controller;
mod keypad_view;
mod logger;
mod models;
mod overlays;
mod panic_display;
mod status_view;
mod text_view;
mod time;

pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Debug;
pub const XOSC_CRYSTAL_FREQ: Hertz = Hertz(12_000_000);

type LedPin = hal::gpio::Pin<hal::gpio::pin::bank0::Gpio13, hal::gpio::PushPullOutput>;
type UsbState = (
    UsbDevice<'static, hal::usb::UsbBus>,
    UsbHidClass<
        hal::usb::UsbBus,
        HList!(
            ConsumerControlInterface<'static, hal::usb::UsbBus>,
            WheelMouseInterface<'static, hal::usb::UsbBus>,
            NKROBootKeyboardInterface<'static, hal::usb::UsbBus, TimerClock>,
        ),
    >,
);
pub type DynInputPin = dyn InputPin<Error = Infallible>;

static LOGGER: Logger = Logger::default();
static SYSTICK_STATE: Mutex<Cell<Option<LedPin>>> = Mutex::new(Cell::new(None));
static USBCTRL_SHARED: Mutex<RefCell<Option<UsbState>>> = Mutex::new(RefCell::new(None));

static KEYBOARD_LEDS: AtomicU8 = AtomicU8::new(0);
static KEYS: DebouncedInputArray<13> = DebouncedInputArray::new();

#[entry]
fn main() -> ! {
    //Safety: no interrupts enabled
    unsafe {
        log::set_logger_racy(&LOGGER)
            .map(|()| log::set_max_level(MAX_LOG_LEVEL))
            .unwrap();
    }

    log::info!("Starting");

    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        crate::XOSC_CRYSTAL_FREQ.integer(),
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = hal::Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    //display spi
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
        Hertz::new(16_000_000u32),
        &embedded_hal::spi::MODE_0,
    );

    let mut display: GraphicsMode<_> = sh1106::Builder::new()
        .connect_spi(oled_spi, oled_dc, oled_cs)
        .into();
    display.init().unwrap();
    display.flush().unwrap();
    log::info!("Display initialised");

    if let Some(msg) = panic_persist::get_panic_message_utf8() {
        //NB never returns
        panic_display::display_and_reboot(msg, display, &pins.button.into_pull_up_input());
    }
    log::info!("No persisted panic");
    static mut CLOCK: Option<TimerClock> = None;
    //Safety: interrupts not enabled yet
    let clock = unsafe {
        CLOCK = Some(TimerClock::new(hal::Timer::new(pac.TIMER, &mut pac.RESETS)));
        CLOCK.as_ref().unwrap()
    };

    static mut USB_ALLOC: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

    //Safety: interrupts not enabled yet
    let usb_alloc = unsafe {
        USB_ALLOC = Some(UsbBusAllocator::new(hal::usb::UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        )));
        USB_ALLOC.as_ref().unwrap()
    };

    let usb_hid = UsbHidClassBuilder::new()
        .add_interface(
            usbd_hid_devices::device::keyboard::NKROBootKeyboardInterface::default_config(clock),
        )
        .add_interface(usbd_hid_devices::device::mouse::WheelMouseInterface::default_config())
        .add_interface(
            usbd_hid_devices::device::consumer::ConsumerControlInterface::default_config(),
        )
        //Build
        .build(usb_alloc);

    let usb_device = UsbDeviceBuilder::new(usb_alloc, UsbVidPid(0x1209, 0x0005))
        .manufacturer("usbd-hid-devices")
        .product("Macropad")
        .serial_number("TEST")
        .supports_remote_wakeup(false)
        .build();

    cortex_m::interrupt::free(|cs| {
        USBCTRL_SHARED
            .borrow(cs)
            .replace(Some((usb_device, usb_hid)));
    });
    //NB Safety - interrupts enabled from this point onwards
    unsafe {
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
    };
    log::info!("USB configured");

    // let encoder_rota = pins.encoder_rota.into_pull_up_input();
    // let encoder_rotb = pins.encoder_rotb.into_pull_up_input();

    KEYS.set_pins([
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
        pins.button.into_pull_up_input().into(),
    ]);

    cortex_m::interrupt::free(|cs| {
        SYSTICK_STATE
            .borrow(cs)
            .set(Some(pins.led.into_push_pull_output()))
    });

    let mut macropad_model = PeripheralsModel::new(clock, &KEYS);
    let mut display_model = DisplayModel::new(display, clock);
    let mut app_model = ApplicationModel::default();
    let mut usb_model = UsbModel::new(&USBCTRL_SHARED, &KEYBOARD_LEDS);

    let keypad_controller = KeypadController::default();
    let display_controller = DisplayController::default();

    //100 mico seconds
    // let reload_value = (clocks.system_clock.freq() / 10_000).integer() - 1;
    let reload_value = 1_000 - 1;
    core.SYST.set_reload(reload_value);
    core.SYST.clear_current();
    //External clock, driven by the Watchdog - 1 tick per us
    core.SYST.set_clock_source(SystClkSource::External);
    core.SYST.enable_interrupt();
    core.SYST.enable_counter();

    log::info!("Interrupts enabled");

    log::info!("Entering main loop");
    loop {
        keypad_controller.tick(&mut macropad_model, &mut app_model, &mut usb_model);
        display_controller.tick(
            &mut display_model,
            &macropad_model,
            &mut app_model,
            &usb_model,
        );
    }
}

#[allow(non_snake_case)]
#[exception]
fn SysTick() {
    static mut LED: Option<LedPin> = None;

    if LED.is_none() {
        *LED = cortex_m::interrupt::free(|cs| SYSTICK_STATE.borrow(cs).take());
    }

    if let Some(led) = LED {
        led.toggle().unwrap();
    }
    KEYS.tick();
    cortex_m::asm::sev();
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| {
        let mut usb_ref = USBCTRL_SHARED.borrow(cs).borrow_mut();
        if usb_ref.is_none() {
            panic!("Keyboard not available to IRQ");

            //return;
        }

        let (ref mut usb_device, ref mut usb_hid) = usb_ref.as_mut().unwrap();
        if usb_device.poll(&mut [usb_hid]) {
            let keyboard = usb_hid.interface::<NKROBootKeyboardInterface<'_, _, _>, _>();
            match keyboard.read_report() {
                Err(UsbError::WouldBlock) => {}
                Err(e) => {
                    panic!("Failed to read keyboard report: {:?}", e)
                }
                Ok(l) => {
                    KEYBOARD_LEDS.store(
                        if l.num_lock { 1 } else { 0 }
                            | if l.caps_lock { 2 } else { 0 }
                            | if l.scroll_lock { 4 } else { 0 }
                            | if l.compose { 8 } else { 0 }
                            | if l.kana { 16 } else { 0 },
                        Ordering::Relaxed,
                    );
                }
            }
        }
    });
    cortex_m::asm::sev();
}
