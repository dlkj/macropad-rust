use embedded_hal::digital::v2::InputPin;

pub struct Macropad<I> {
    keys: [debounce::DebouncedPin<I>; 12],
}

impl<I, PinE> Macropad<I>
where
    I: InputPin<Error = PinE>,
    PinE: core::fmt::Debug,
{
    pub fn new(keys: [I; 12]) -> Macropad<I> {
        Macropad {
            keys: keys.map(|p| debounce::DebouncedPin::new(p, true)),
        }
    }

    pub fn get_keys(&self) -> [bool; 12] {
        self.keys
            .iter()
            .map(|p| p.is_low().unwrap_or(false))
            .collect::<arrayvec::ArrayVec<bool, 12>>()
            .into_inner()
            .expect("Unexpected number of results")
    }

    pub fn update(&mut self) -> Result<(), PinE> {
        for k in &mut self.keys {
            k.update()?;
        }
        Ok(())
    }
}
