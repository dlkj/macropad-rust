use crate::models::keypad_model::KeyState;
use embedded_graphics::mono_font::ascii::FONT_6X13_BOLD;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::Drawable;

pub struct KeypadView<'a> {
    key_presses: &'a [KeyState; 13],
    num_lock: bool,
}

impl<'a> KeypadView<'a> {
    pub(crate) fn new(key_presses: &'a [KeyState; 13], num_lock: bool) -> Self {
        Self {
            key_presses,
            num_lock,
        }
    }

    fn draw_button<D>(
        display: &mut D,
        top_left: Point,
        text: &str,
        color: BinaryColor,
    ) -> Result<<Self as Drawable>::Output, <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = <Self as Drawable>::Color>,
    {
        let led_text_style = MonoTextStyle::new(&FONT_6X13_BOLD, color);
        let fill = PrimitiveStyle::with_fill(color.invert());

        let bounding_box = Rectangle::new(top_left, Size::new(8, 11));

        RoundedRectangle::with_equal_corners(bounding_box, Size::new(2, 2))
            .into_styled(fill)
            .draw(display)?;

        Text::with_alignment(
            text,
            bounding_box.center() + Point::new(0, 4),
            led_text_style,
            Alignment::Center,
        )
        .draw(display)?;
        Ok(())
    }
}

impl<'a> Drawable for KeypadView<'a> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        Self::draw_button(
            display,
            Point::new(0, 0),
            if self.num_lock { "7" } else { "H" },
            BinaryColor::from(self.key_presses[0] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(10, 0),
            if self.num_lock { "8" } else { "^" },
            BinaryColor::from(self.key_presses[1] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(20, 0),
            if self.num_lock { "9" } else { "U" },
            BinaryColor::from(self.key_presses[2] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(0, 13),
            if self.num_lock { "4" } else { "<" },
            BinaryColor::from(self.key_presses[3] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(10, 13),
            if self.num_lock { "5" } else { " " },
            BinaryColor::from(self.key_presses[4] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(20, 13),
            if self.num_lock { "6" } else { ">" },
            BinaryColor::from(self.key_presses[5] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(0, 26),
            if self.num_lock { "1" } else { "E" },
            BinaryColor::from(self.key_presses[6] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(10, 26),
            if self.num_lock { "2" } else { "v" },
            BinaryColor::from(self.key_presses[7] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(20, 26),
            if self.num_lock { "3" } else { "D" },
            BinaryColor::from(self.key_presses[8] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(0, 39),
            if self.num_lock { "0" } else { "I" },
            BinaryColor::from(self.key_presses[9] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(10, 39),
            if self.num_lock { "." } else { "D" },
            BinaryColor::from(self.key_presses[10] == KeyState::Down),
        )?;
        Self::draw_button(
            display,
            Point::new(20, 39),
            "E",
            BinaryColor::from(self.key_presses[11] == KeyState::Down),
        )?;

        Ok(())
    }
}
