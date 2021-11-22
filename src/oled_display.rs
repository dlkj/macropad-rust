use embedded_graphics::{
    image::{Image, ImageRawLE},
    mono_font::{ascii::FONT_4X6, ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
use sh1106::interface::DisplayInterface;
use sh1106::prelude::GraphicsMode;

pub struct OledDisplay<DI>
where
    DI: sh1106::interface::DisplayInterface,
{
    display: GraphicsMode<DI>,
}

impl<DI: DisplayInterface> OledDisplay<DI> {
    pub fn new(display: GraphicsMode<DI>) -> OledDisplay<DI> {
        return OledDisplay { display };
    }

    pub fn draw_image(&mut self, data: &[u8], width: u32) -> Result<(), DI::Error> {
        self.display.clear();

        let img: ImageRawLE<BinaryColor> = ImageRawLE::new(data, width);
        Image::new(&img, Point::new(32, 0))
            .draw(&mut self.display)
            .unwrap();

        self.display.flush()?;

        Ok(())
    }

    pub fn draw_text(&mut self, text: &str) -> Result<(), DI::Error> {
        self.display.clear();
        let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        Text::with_alignment(text, Point::new(0, 6), character_style, Alignment::Left)
            .draw(&mut self.display)
            .unwrap();
        self.display.flush()?;

        Ok(())
    }

    pub fn draw_test(&mut self) -> Result<(), DI::Error> {
        self.display.clear();

        // Create styles used by the drawing operations.
        let thin_stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
        let thick_stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 3);
        let border_stroke = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(3)
            .stroke_alignment(StrokeAlignment::Inside)
            .build();
        let fill = PrimitiveStyle::with_fill(BinaryColor::On);
        let character_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

        let yoffset = 10;

        // Draw a 3px wide outline around the display.
        self.display
            .bounding_box()
            .into_styled(border_stroke)
            .draw(&mut self.display)
            .unwrap();

        // Draw a triangle.
        Triangle::new(
            Point::new(16, 16 + yoffset),
            Point::new(16 + 16, 16 + yoffset),
            Point::new(16 + 8, yoffset),
        )
        .into_styled(thin_stroke)
        .draw(&mut self.display)
        .unwrap();

        // Draw a filled square
        Rectangle::new(Point::new(52, yoffset), Size::new(16, 16))
            .into_styled(fill)
            .draw(&mut self.display)
            .unwrap();

        // Draw a circle with a 3px wide stroke.
        Circle::new(Point::new(88, yoffset), 17)
            .into_styled(thick_stroke)
            .draw(&mut self.display)
            .unwrap();

        // Draw centered text.
        let text = "embedded-graphics";
        Text::with_alignment(
            text,
            self.display.bounding_box().center() + Point::new(0, 15),
            character_style,
            Alignment::Center,
        )
        .draw(&mut self.display)
        .unwrap();

        self.display.flush()?;

        Ok(())
    }
}
