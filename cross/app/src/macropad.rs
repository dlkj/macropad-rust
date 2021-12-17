use embedded_hal::digital::v2::InputPin;

pub struct Macropad<I> {
    keys: [debounce::DebouncedPin<I>; 12],
    key_map: [u8; 12],
}

impl<I, PinE> Macropad<I>
where
    I: InputPin<Error = PinE>,
    PinE: core::fmt::Debug,
{
    pub fn new(keys: [I; 12], key_map: [u8; 12]) -> Macropad<I> {
        Macropad {
            keys: keys.map(|p| debounce::DebouncedPin::new(p, true)),
            key_map,
        }
    }

    pub fn get_keycodes(&self) -> arrayvec::ArrayVec<u8, 12> {
        self.keys
            .iter()
            .zip(self.key_map.iter())
            .flat_map(|(p, k)| p.is_low().ok().and_then(|v| v.then(|| *k)))
            .collect()
    }

    pub fn update(&mut self) -> Result<(), PinE> {
        for k in &mut self.keys {
            k.update()?;
        }
        Ok(())
    }
}
