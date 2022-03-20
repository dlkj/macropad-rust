use core::fmt::Write;

use crate::keypad_controller::KeyState;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use usb_device::device::UsbDeviceState;

pub struct StatusView<'a> {
    ticks: u64,
    key_values: &'a [KeyState; 13],
    keyboard_leds: u8,
    usb_state: UsbDeviceState,
}

impl<'a> StatusView<'a> {
    pub fn new(
        ticks: u64,
        key_values: &'a [KeyState; 13],
        keyboard_leds: u8,

        usb_state: UsbDeviceState,
    ) -> Self {
        Self {
            ticks,
            key_values,
            keyboard_leds,
            usb_state,
        }
    }
}

impl<'a> Drawable for StatusView<'a> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<Self::Output, <D as DrawTarget>::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let mut buffer = heapless::String::<512>::new();
        write!(
            &mut buffer,
            "keys: {:?}\nusb: {:?}\nleds: {:02X}\nclock: {}",
            self.key_values, self.usb_state, self.keyboard_leds, self.ticks
        )
        .unwrap();

        let character_style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
        Text::new(&buffer, Point::new(0, 12), character_style)
            .draw(display)
            .map(|_| ())
    }
}
