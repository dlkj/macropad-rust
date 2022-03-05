use crate::LOGGER;
use core::convert::Infallible;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::Drawable;
use embedded_hal::digital::v2::InputPin;
use embedded_time::duration::Milliseconds;
use embedded_time::timer::param::{Periodic, Running};
use embedded_time::{Clock, Timer};
use heapless::String;
use sh1106::interface::DisplayInterface;
use sh1106::prelude::*;

const DISPLAY_UPDATE: Milliseconds = Milliseconds(20);

pub struct MacropadModel<'a, DI: DisplayInterface, C: Clock<T = u32>> {
    display: GraphicsMode<DI>,
    clock: &'a C,
    display_update_timer: Timer<'a, Periodic, Running, C, Milliseconds>,
    display_mode_pin: &'a dyn InputPin<Error = Infallible>,
}

impl<'a, DI: DisplayInterface, C: Clock<T = u32>> MacropadModel<'a, DI, C> {
    pub fn log(&self) -> String<512> {
        LOGGER.log_buffer()
    }

    pub fn ticks_since_epoc(&self) -> u32 {
        self.clock
            .try_now()
            .unwrap()
            .duration_since_epoch()
            .integer()
    }

    pub fn display_draw<V: Drawable<Color = BinaryColor>>(&mut self, view: V) {
        self.display.clear();
        view.draw(&mut self.display).unwrap();
        self.display.flush().ok();
    }

    pub fn display_update_due(&mut self) -> bool {
        self.display_update_timer.period_complete().unwrap()
    }

    pub fn display_mode(&self) -> DisplayMode {
        if self.display_mode_pin.is_high().unwrap() {
            DisplayMode::Time
        } else {
            DisplayMode::Log
        }
    }

    pub fn new(
        display: GraphicsMode<DI>,
        clock: &'a C,
        display_mode_pin: &'a dyn InputPin<Error = Infallible>,
    ) -> Self {
        Self {
            display,
            display_update_timer: clock
                .new_timer(DISPLAY_UPDATE)
                .into_periodic()
                .start()
                .unwrap(),
            clock,
            display_mode_pin,
        }
    }
}

pub enum DisplayMode {
    Log,
    Time,
}
