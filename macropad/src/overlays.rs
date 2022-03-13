use core::fmt::Write;
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_time::duration::Microseconds;

pub struct TimingOverlayView {
    display_time: Microseconds,
    keypad_time: Microseconds,
    f: u8,
}

impl TimingOverlayView {
    pub(crate) fn new(display_time: Microseconds, keypad_time: Microseconds, f: u8) -> Self {
        Self {
            display_time,
            keypad_time,
            f,
        }
    }
}

impl Drawable for TimingOverlayView {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let mut buffer = heapless::String::<64>::new();
        write!(
            &mut buffer,
            "k: {} ns\nd: {} ns\n{}",
            self.keypad_time, self.display_time, self.f
        )
        .unwrap();

        let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        Text::new(&buffer, Point::new(75, 45), character_style).draw(display)?;
        Ok(())
    }
}
