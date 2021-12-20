use embedded_hal::digital::v2::InputPin;

pub struct RotaryEncoder<P> {
    pin_a: P,
    pin_b: P,
    state: u8,
    quarter_idx: i8,
    value: i32,
}

impl<P> RotaryEncoder<P>
where
    P: InputPin,
    P::Error: core::fmt::Debug,
{
    pub fn new(pin_a: P, pin_b: P) -> RotaryEncoder<P> {
        RotaryEncoder {
            pin_a,
            pin_b,
            state: 3,
            quarter_idx: 0,
            value: 0,
        }
    }

    pub fn update(&mut self) {
        const ENCODER_STATES: [i8; 16] = [0, -1, 1, 0, 1, 0, 0, -1, -1, 0, 0, 1, 0, 1, -1, 0];

        let new_state = self.pin_a.is_high().expect("Unable to read pin_a") as u8
            | (self.pin_b.is_high().expect("unable to read pin_b") as u8 * 2);

        let transision = ENCODER_STATES[((new_state << 2) | self.state) as usize];

        self.state = new_state;
        self.quarter_idx += transision;

        if self.quarter_idx > 3 {
            self.value -= 1;
            self.quarter_idx -= 4;
        } else if self.quarter_idx < -3 {
            self.value += 1;
            self.quarter_idx += 4;
        }
    }

    #[allow(dead_code)]
    pub fn pins_borrow(&self) -> (&P, &P) {
        (&self.pin_a, &self.pin_b)
    }

    pub fn pins_borrow_mut(&mut self) -> (&mut P, &mut P) {
        (&mut self.pin_a, &mut self.pin_b)
    }

    pub fn value(&self) -> i32 {
        self.value
    }
}
