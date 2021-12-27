use arrayvec::ArrayVec;
use bitflags::bitflags;
use embedded_hal::digital::v2::InputPin;

type KeyCode = u8;
type LayerID = u8;

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
        const NONE        = 0b00000000;
    }
}

struct KeyState {}

pub trait KeyboardMatrix<const KEY_COUNT: usize> {
    fn keys(&self) -> [KeyState; KEY_COUNT];
}

struct DirectPinMatrix<P, const N: usize> {
    pins: [P; N],
}

impl<P, const N: usize> DirectPinMatrix<P, N> {
    fn new(pins: [P; N]) -> DirectPinMatrix<P, N>
    where
        P: InputPin,
    {
        DirectPinMatrix { pins }
    }
}

impl<P, const N: usize> KeyboardMatrix<N> for DirectPinMatrix<P, N>
where
    P: InputPin,
{
    fn keys(&self) -> [KeyState; N] {
        let mut keystates = [KeyState {}; N];

        for (i, p) in self.pins.iter().enumerate() {
            keystates[i].pressed = p.is_low()?;
        }

        keystates
    }
}

pub trait KeyboardLayout<const KEY_COUNT: usize> {}

pub struct KeyboardState<const KEY_COUNT: usize> {
    modifiers: Modifiers,
    keycodes: ArrayVec<KeyCode, KEY_COUNT>,
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
    pub fn update(&mut self) {}
    pub fn state(&self) -> KeyboardState<KEY_COUNT> {
        KeyboardState {
            modifiers: Modifiers::NONE,
            keycodes: ArrayVec::new(),
        }
    }
}
