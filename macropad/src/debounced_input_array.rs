use crate::hal::gpio::DynPin;
use crate::DebouncedInputPin;
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use embedded_hal::digital::v2::{InputPin, PinState};

pub struct DebouncedInputArray<const N: usize> {
    pins: Mutex<RefCell<Option<[DebouncedInputPin; N]>>>,
}

impl<const N: usize> DebouncedInputArray<N> {
    pub(crate) fn set_pins(&self, pins: [DynPin; N]) {
        let debounced_pins = pins.map(DebouncedInputPin::new);
        cortex_m::interrupt::free(|cs| {
            self.pins.borrow(cs).replace(Some(debounced_pins));
        });
    }

    pub const fn new() -> Self {
        Self {
            pins: Mutex::new(RefCell::new(None)),
        }
    }

    pub fn tick(&self) {
        cortex_m::interrupt::free(|cs| {
            if let Some(pins) = self.pins.borrow(cs).borrow_mut().as_mut() {
                for p in pins.iter_mut() {
                    p.update();
                }
            }
        })
    }

    pub(crate) fn values(&self) -> [PinState; N] {
        cortex_m::interrupt::free(|cs| {
            let mut values = [PinState::Low; N];
            if let Some(pins) = self.pins.borrow(cs).borrow().as_ref() {
                for (i, p) in pins.iter().enumerate() {
                    values[i] = PinState::from(p.is_high().unwrap());
                }
            }
            values
        })
    }
}
