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

    pub fn get_keys(&self) -> arrayvec::ArrayVec<usize, 12> {
        self.keys
            .iter()
            .enumerate()
            .flat_map(|(i, p)| p.is_low().ok().and_then(|v| v.then(|| i)))
            .collect()
    }

    pub fn update(&mut self) -> Result<(), PinE> {
        for k in &mut self.keys {
            k.update()?;
        }
        Ok(())
    }
}
