#![no_std]

pub enum DebouncerState {
    Low,
    High,
    Unknown,
}

pub struct Debouncer<P> {
    pin: P,
    last: bool,
    history: u8,
}

impl<P, E> Debouncer<P>
where
    P: embedded_hal::digital::v2::InputPin<Error = E>,
{
    pub fn new(pin: P, default_state: bool) -> Debouncer<P> {
        Debouncer {
            pin,
            last: default_state,
            history: if default_state { u8::MAX } else { 0 },
        }
    }

    pub fn update(&mut self) -> Result<(), E> {
        self.history = (self.history << 1) | if self.pin.is_high()? { 1 } else { 0 };

        self.last = match self.history {
            u8::MAX => true,
            0 => false,
            _ => self.last,
        };

        Ok(())
    }
}

impl<P> embedded_hal::digital::v2::InputPin for Debouncer<P>
where
    P: embedded_hal::digital::v2::InputPin,
{
    type Error = P::Error;

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(self.last)
    }
    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(!self.last)
    }
}

#[cfg(test)]
mod tests;
