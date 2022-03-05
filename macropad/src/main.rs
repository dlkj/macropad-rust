#![no_std]
#![no_main]

use core::cell::Cell;

use adafruit_macropad::{
    hal::{
        self as hal,
        clocks::Clock,
        pac::{self},
    },
    Pins,
};
use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::entry;
use cortex_m_rt::exception;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::digital::v2::ToggleableOutputPin;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Hertz;
use log::LevelFilter;
use panic_persist as _;
use sh1106::prelude::*;

use crate::display_controller::DisplayController;
use crate::logger::Logger;
use crate::macropad_model::MacropadModel;
use crate::timer_clock::TimerClock;

mod display_controller;
mod led_controller;
mod logger;
mod macropad_model;
mod number_view;
mod panic_display;
mod text_view;
mod timer_clock;

pub const MAX_LOG_LEVEL: LevelFilter = LevelFilter::Debug;
pub const XOSC_CRYSTAL_FREQ: Hertz = Hertz(12_000_000);

type LedPin = hal::gpio::Pin<hal::gpio::pin::bank0::Gpio13, hal::gpio::PushPullOutput>;

static LOGGER: Logger = Logger::default();
static SYSTICK_STATE: Mutex<Cell<Option<LedPin>>> = Mutex::new(Cell::new(None));

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

    let button = pins.button.into_pull_up_input();
    let key12 = pins.key12.into_pull_up_input();

    if let Some(msg) = panic_persist::get_panic_message_utf8() {
        //NB never returns
        panic_display::display_and_reboot(msg, display, &button);
    }
    log::info!("No persisted panic");

    let clock = TimerClock::new(hal::Timer::new(pac.TIMER, &mut pac.RESETS));

    cortex_m::interrupt::free(|cs| {
        SYSTICK_STATE
            .borrow(cs)
            .set(Some(pins.led.into_push_pull_output()))
    });

    let macropad_model = MacropadModel::new(display, &clock, &key12);
    let mut macropad_controller = DisplayController::new(macropad_model);

    //100 mico seconds
    // let reload_value = (clocks.system_clock.freq() / 10_000).integer() - 1;
    let reload_value = 1_000 - 1;
    core.SYST.set_reload(reload_value);
    core.SYST.clear_current();
    //External clock, driven by the Watchdog - 1 tick per us
    core.SYST.set_clock_source(SystClkSource::External);
    core.SYST.enable_interrupt();
    core.SYST.enable_counter();
    //NB Safety - interrupts enabled from this point onwards
    log::info!("Timer enabled");

    log::info!("Entering main loop");
    loop {
        if button.is_low().unwrap() {
            //USB boot with pin 13 for usb activity
            hal::rom_data::reset_to_usb_boot(0x1 << 13, 0x0);
        }

        macropad_controller.tick();
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
}
