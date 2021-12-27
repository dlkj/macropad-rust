use arrayvec::ArrayVec;
use bitflags::bitflags;
use debounce::DebouncedPin;
use embedded_hal::digital::v2::InputPin;

type KeyCode = u8;

bitflags! {
    pub struct Modifiers: u8 {
        const CTRL_LEFT   = 0b00000001;
        const SHIFT_LEFT  = 0b00000010;
        const ALT_LEFT    = 0b00000100;
        const GUI_LEFT    = 0b00001000;
        const CTRL_RIGHT  = 0b00010000;
        const SHIFT_RIGHT = 0b00100000;
        const ALT_RIGHT   = 0b01000000;
        const GUI_RIGHT   = 0b10000000;
    }
}

#[derive(Default, Copy, Clone)]
pub struct KeyState {
    pub pressed: bool,
}

pub trait KeyboardMatrix<const KEY_COUNT: usize> {
    type Error;
    fn update(&mut self) -> Result<(), Self::Error>;
    fn keys(&self) -> Result<[KeyState; KEY_COUNT], Self::Error>;
}

pub struct DirectPinMatrix<P, const N: usize> {
    pins: [DebouncedPin<P>; N],
}

impl<P, const N: usize> DirectPinMatrix<P, N> {
    pub fn new(pins: [P; N]) -> DirectPinMatrix<P, N>
    where
        P: InputPin,
    {
        DirectPinMatrix {
            pins: pins.map(|p| DebouncedPin::new(p, true)),
        }
    }
}

impl<P, const N: usize> KeyboardMatrix<N> for DirectPinMatrix<P, N>
where
    P: InputPin,
{
    type Error = P::Error;

    fn keys(&self) -> Result<[KeyState; N], Self::Error> {
        let mut keystates = [KeyState::default(); N];

        for (i, p) in self.pins.iter().enumerate() {
            keystates[i].pressed = p.is_low()?;
        }
        Ok(keystates)
    }

    fn update(&mut self) -> Result<(), Self::Error> {
        for p in &mut self.pins {
            p.update()?;
        }
        Ok(())
    }
}

pub trait KeyboardLayout<const KEY_COUNT: usize> {}

pub struct BasicKeyboardLayout<const N: usize> {}

impl<const N: usize> KeyboardLayout<N> for BasicKeyboardLayout<N> {}

pub struct KeyboardState<const KEY_COUNT: usize> {
    pub modifiers: Modifiers,
    pub keycodes: ArrayVec<KeyCode, KEY_COUNT>,
}

pub struct Keyboard<KM, KL, const KEY_COUNT: usize> {
    matrix: KM,
    _layout: KL,
}

impl<KM, KL, const KEY_COUNT: usize> Keyboard<KM, KL, KEY_COUNT>
where
    KM: KeyboardMatrix<KEY_COUNT>,
    KL: KeyboardLayout<KEY_COUNT>,
{
    pub fn new(matrix: KM, layout: KL) -> Keyboard<KM, KL, KEY_COUNT> {
        Keyboard {
            matrix,
            _layout: layout,
        }
    }
    pub fn update(&mut self) -> Result<(), KM::Error> {
        self.matrix.update()
    }
    pub fn state(&self) -> KeyboardState<KEY_COUNT> {
        KeyboardState {
            modifiers: Modifiers::empty(),
            keycodes: self
                .matrix
                .keys()
                .unwrap_or([KeyState { pressed: false }; KEY_COUNT])
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.pressed.then(|| i as u8))
                .collect(),
        }
    }
}
