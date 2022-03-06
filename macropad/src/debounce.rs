use crate::hal::gpio::{DynInput, DynPin, DynPinMode};
use core::convert::Infallible;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::PinState;

pub struct DebouncedInputPin {
    pin: DynPin,
    last: bool,
    history: u8,
}

impl DebouncedInputPin {
    pub fn new(pin: DynPin) -> Self {
        let default_value = match pin.mode() {
            DynPinMode::Input(c) => match c {
                DynInput::PullUp => PinState::High,
                DynInput::PullDown => PinState::Low,
                _ => panic!("Invalid pin configuration"),
            },
            _ => panic!("Invalid pin mode"),
        };

        Self {
            pin,
            last: match default_value {
                PinState::High => true,
                PinState::Low => false,
            },
            history: match default_value {
                PinState::High => u8::MAX,
                PinState::Low => u8::MIN,
            },
        }
    }

    pub fn update(&mut self) {
        const MASK: u8 = 0b11100000; //look for 5 stable values

        self.history = (self.history << 1) | if self.pin.is_high().unwrap() { 1 } else { 0 } | MASK;

        self.last = match self.history {
            u8::MAX => true,
            MASK => false,
            _ => self.last,
        };
    }
}

impl embedded_hal::digital::v2::InputPin for DebouncedInputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.last)
    }
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(!self.last)
    }
}
