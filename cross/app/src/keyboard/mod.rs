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

pub struct KeyboardLayoutState<const KEY_COUNT: usize> {
    pub modifiers: Modifiers,
    pub keycodes: ArrayVec<KeyCode, KEY_COUNT>,
}

pub trait KeyboardLayout<const N: usize> {
    fn state(&self, keys: &[KeyState; N]) -> KeyboardLayoutState<N>;
}

pub struct BasicKeyboardLayout<const N: usize> {
    keymap: [KeyCode; N],
}

impl<const N: usize> BasicKeyboardLayout<N> {
    pub fn new(keymap: [KeyCode; N]) -> BasicKeyboardLayout<N> {
        BasicKeyboardLayout { keymap }
    }
}

impl<const N: usize> KeyboardLayout<N> for BasicKeyboardLayout<N> {
    fn state(&self, keys: &[KeyState; N]) -> KeyboardLayoutState<N> {
        let keycodes = keys
            .iter()
            .enumerate()
            .filter_map(|(i, k)| k.pressed.then(|| self.keymap[i]))
            .collect();

        KeyboardLayoutState {
            modifiers: Modifiers::empty(),
            keycodes,
        }
    }
}

pub struct KeyboardState<const KEY_COUNT: usize> {
    pub modifiers: Modifiers,
    pub keycodes: ArrayVec<KeyCode, KEY_COUNT>,
    pub keys: [KeyState; KEY_COUNT],
}

pub struct Keyboard<KM, KL, const KEY_COUNT: usize> {
    matrix: KM,
    layout: KL,
}

impl<KM, KL, const KEY_COUNT: usize> Keyboard<KM, KL, KEY_COUNT>
where
    KM: KeyboardMatrix<KEY_COUNT>,
    KL: KeyboardLayout<KEY_COUNT>,
{
    pub fn new(matrix: KM, layout: KL) -> Keyboard<KM, KL, KEY_COUNT> {
        Keyboard { matrix, layout }
    }
    pub fn update(&mut self) -> Result<(), KM::Error> {
        self.matrix.update()
    }
    pub fn state(&self) -> Result<KeyboardState<KEY_COUNT>, KM::Error> {
        let keys = self.matrix.keys()?;
        let layout_state = self.layout.state(&keys);

        Ok(KeyboardState {
            modifiers: layout_state.modifiers,
            keycodes: layout_state.keycodes,
            keys,
        })
    }
}
