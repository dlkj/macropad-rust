use core::fmt::Write;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Drawable;
use embedded_text::alignment::{HorizontalAlignment, VerticalAlignment};
use embedded_text::style::{HeightMode, TextBoxStyleBuilder, VerticalOverdraw};
use embedded_text::TextBox;

pub struct NumberView {
    number: u32,
}

impl NumberView {
    pub fn new(ticks: u32) -> Self {
        Self { number: ticks }
    }
}

impl Drawable for NumberView {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let mut buffer = heapless::String::<32>::new();
        write!(&mut buffer, "{}", self.number).unwrap();

        let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        let text_box_style = TextBoxStyleBuilder::new()
            .height_mode(HeightMode::Exact(VerticalOverdraw::FullRowsOnly))
            .alignment(HorizontalAlignment::Left)
            .vertical_alignment(VerticalAlignment::Bottom)
            .build();
        let bounds = Rectangle::new(Point::zero(), Size::new(128, 64));
        let text_box =
            TextBox::with_textbox_style(&buffer, bounds, character_style, text_box_style);

        text_box.draw(display).map(|_| ())
    }
}
