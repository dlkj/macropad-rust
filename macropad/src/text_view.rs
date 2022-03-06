use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Drawable;
use embedded_text::alignment::VerticalAlignment;
use embedded_text::style::TextBoxStyleBuilder;
use embedded_text::TextBox;
use heapless::String;

pub struct TextView<const N: usize> {
    text: String<N>,
}

impl<const N: usize> TextView<N> {
    pub fn new(text: String<N>) -> Self {
        Self { text }
    }
}

impl<const N: usize> Drawable for TextView<N> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        //we can only draw 9 lines, so we can find the last 8 \n and truncate the text
        //we could also look for long line and factor these into the truncation
        let text = if let Some((i, _)) = self.text.rmatch_indices('\n').nth(10) {
            &self.text.as_str()[i..self.text.len() - 1]
        } else {
            //truncate tailing new line
            &self.text.as_str()[..self.text.len() - 1]
        };

        let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);

        let text_box_style = TextBoxStyleBuilder::new()
            .vertical_alignment(VerticalAlignment::Bottom)
            .build();
        let bounds = Rectangle::new(Point::zero(), Size::new(128, 64));
        TextBox::with_textbox_style(text, bounds, character_style, text_box_style)
            .draw(display)
            .map(|_| ())
    }
}
