use arrayvec::ArrayVec;
use bitflags::bitflags;

type KeyID = u8;
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

pub struct KeyboardMatrix {}
pub struct KeyboardLayout<const KEY_COUNT: usize> {}

pub struct KeyboardState<const KEY_COUNT: usize> {
    modifiers: Modifiers,
    keys: ArrayVec<KeyID, KEY_COUNT>,
    keycodes: ArrayVec<KeyCode, KEY_COUNT>,
    active_layer: LayerID,
}

pub struct Keyboard<const KEY_COUNT: usize> {
    matrix: KeyboardMatrix,
    layout: KeyboardLayout<KEY_COUNT>,
    state: KeyboardState<KEY_COUNT>,
}

impl<const KEY_COUNT: usize> Keyboard<KEY_COUNT> {
    pub fn new(matrix: KeyboardMatrix, layout: KeyboardLayout<KEY_COUNT>) -> Keyboard<KEY_COUNT> {
        Keyboard {
            matrix,
            layout,
            state: KeyboardState {
                modifiers: Modifiers::NONE,
                keys: ArrayVec::new(),
                keycodes: ArrayVec::new(),
                active_layer: 0,
            },
        }
    }
    pub fn update(&mut self) {}
    pub fn borrow_state(&self) -> &KeyboardState<KEY_COUNT> {
        &self.state
    }
}
