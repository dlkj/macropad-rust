use core::cell::RefCell;
use embedded_hal::digital::v2::InputPin;

pub struct RotaryEncoder<'a, P> {
    pin_a: &'a RefCell<P>,
    pin_b: &'a RefCell<P>,
    state: u8,
    value: i32,
}

impl<P> RotaryEncoder<'_, P>
where
    P: InputPin,
    P::Error: core::fmt::Debug,
{
    pub fn new<'a>(pin_a: &'a RefCell<P>, pin_b: &'a RefCell<P>) -> RotaryEncoder<'a, P> {
        RotaryEncoder {
            pin_a,
            pin_b,
            state: 0,
            value: 0,
        }
    }

    pub fn update(&mut self) {
        const ENCODER_STATES: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];

        self.state = ((self.state << 2)
            | self.pin_a.borrow().is_high().expect("Unable to read pin_a") as u8
            | (self.pin_b.borrow().is_high().expect("unable to read pin_b") as u8 * 2))
            & 0xf;

        self.value += ENCODER_STATES[self.state as usize] as i32;
    }

    pub fn value(&self) -> i32 {
        self.value
    }
}
