use core::fmt::Write;

use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_time::{Clock, Instant};

pub struct ScreensaverView<C: Clock<T = u64>> {
    now: Instant<C>,
}

impl<C: Clock<T = u64>> ScreensaverView<C> {
    pub fn new(now: Instant<C>) -> Self {
        Self { now }
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
        write!(&mut buffer, "{:?}", ticks).unwrap();

        let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        Text::new(
            &buffer,
            Point::new(0, i32::try_from((ticks >> 20) % 64).unwrap()),
            character_style,
        )
        .draw(display)
        .map(|_| ())
    }
}
