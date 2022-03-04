use core::convert::Infallible;

use adafruit_macropad::hal;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_hal::digital::v2::InputPin;
use embedded_text::alignment::{HorizontalAlignment, VerticalAlignment};
use embedded_text::style::{HeightMode, TextBoxStyleBuilder, VerticalOverdraw};
use embedded_text::TextBox;
use sh1106::prelude::*;

pub fn display_and_reboot<DI, E>(msg: &str, mut display: GraphicsMode<DI>, reboot_pin: &dyn InputPin<Error = Infallible>) -> !
    where
        DI: sh1106::interface::DisplayInterface<Error = E>,
        E: core::fmt::Debug
{
    display.clear();

    let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
    let text_box_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::Exact(VerticalOverdraw::FullRowsOnly))
        .alignment(HorizontalAlignment::Left)
        .vertical_alignment(VerticalAlignment::Bottom)
        .build();
    let bounds = Rectangle::new(Point::zero(), Size::new(128, 64));
    let text_box = TextBox::with_textbox_style(msg, bounds, character_style, text_box_style);

    text_box.draw(&mut display).unwrap();
    display.flush().unwrap();

    while reboot_pin.is_high().unwrap_or(false) {
        cortex_m::asm::nop()
    }

    //USB boot with pin 13 for usb activity
    hal::rom_data::reset_to_usb_boot(0x1 << 13, 0x0);
    loop {
        cortex_m::asm::nop()
    }
}
