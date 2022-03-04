#![no_std]
#![no_main]

use core::cell::Cell;
use core::fmt::Write;

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
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::digital::v2::ToggleableOutputPin;
use embedded_text::alignment::{HorizontalAlignment, VerticalAlignment};
use embedded_text::style::{HeightMode, TextBoxStyleBuilder, VerticalOverdraw};
use embedded_text::TextBox;
use embedded_time::duration::Seconds;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Hertz;
use embedded_time::Clock as _;
use log::LevelFilter;
use panic_persist as _;
use sh1106::prelude::*;

use crate::logger::Logger;
use crate::timer_clock::TimerClock;

mod display_controller;
mod keypad_model;
mod led_controller;
mod log_model;
mod logger;
mod macropad_controller;
mod macropad_model;
mod panic_display;
mod rotary_encoder_model;
mod sound_controller;
mod timer_clock;
mod usb_model;

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

    let button = pins.button.into_pull_up_input();
    if let Some(msg) = panic_persist::get_panic_message_utf8() {
        //NB never returns
        panic_display::display_and_reboot(msg, display, &button);
    }

    let clock = TimerClock::new(hal::Timer::new(pac.TIMER, &mut pac.RESETS));

    cortex_m::interrupt::free(|cs| {
        SYSTICK_STATE
            .borrow(cs)
            .set(Some(pins.led.into_push_pull_output()))
    });

    //100 mico seconds
    // let reload_value = (clocks.system_clock.freq() / 10_000).integer() - 1;
    let reload_value = 100_000 - 1;
    core.SYST.set_reload(reload_value);
    core.SYST.clear_current();
    //External clock, driven by the Watchdog - 1 tick per us
    core.SYST.set_clock_source(SystClkSource::External);
    core.SYST.enable_interrupt();
    core.SYST.enable_counter();
    //NB Safety - interrupts enabled from this point onwards

    let mut second_timer = clock
        .new_timer(Seconds(1u32))
        .into_periodic()
        .start()
        .unwrap();
    let mut seconds = 0u32;

    loop {
        cortex_m::asm::nop();
        if button.is_low().unwrap() {
            //USB boot with pin 13 for usb activity
            hal::rom_data::reset_to_usb_boot(0x1 << 13, 0x0);
        }

        if second_timer.period_complete().unwrap() {
            seconds += 1;
            display.clear();

            let mut buffer = heapless::String::<16>::new();
            write!(&mut buffer, "{}", seconds).unwrap();

            let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
            let text_box_style = TextBoxStyleBuilder::new()
                .height_mode(HeightMode::Exact(VerticalOverdraw::FullRowsOnly))
                .alignment(HorizontalAlignment::Left)
                .vertical_alignment(VerticalAlignment::Bottom)
                .build();
            let bounds = Rectangle::new(Point::zero(), Size::new(128, 64));
            let text_box =
                TextBox::with_textbox_style(&buffer, bounds, character_style, text_box_style);

            text_box.draw(&mut display).unwrap();
            display.flush().unwrap();
        }
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
        panic!("0\n1\n2\n3\n4\n5\n6\n7\n8\n")
    }
}
