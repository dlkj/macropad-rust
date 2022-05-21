use core::fmt::Write;

use embedded_graphics::mono_font::ascii::{FONT_4X6, FONT_5X8};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_time::{Clock, Instant};

pub struct ScreensaverView<C: Clock<T = u64>> {
    now: Instant<C>,
    keyboard_leds: u8,
}

impl<C: Clock<T = u64>> ScreensaverView<C> {
    pub fn new(now: Instant<C>, keyboard_leds: u8) -> Self {
        Self { now, keyboard_leds }
    }
}

impl<C: Clock<T = u64>> Drawable for ScreensaverView<C> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let mut buffer = heapless::String::<512>::new();
        let ticks = self.now.duration_since_epoch().integer();
        write!(
            &mut buffer,
            "{}",
            if self.keyboard_leds & 0x1 == 0 {
                "DIR"
            } else {
                "NUM"
            }
        )
        .unwrap();

        let character_style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        Text::new(
            &buffer,
            Point::new(
                ((ticks >> 18) % 97).try_into().unwrap(),
                (8 + (ticks >> 18) % 53).try_into().unwrap(),
            ),
            character_style,
        )
        .draw(display)
        .map(|_| ())
    }
}
