use crate::keypad_controller::{Action, AppAction};
use embedded_time::duration::{Generic, Milliseconds};
use embedded_time::fixed_point::FixedPoint;
use embedded_time::{Clock, Instant};
use heapless::Vec;
use num_traits::Bounded;
use usbd_human_interface_device::page::Keyboard;

const KEY_COUNT: usize = 13;

pub struct KeypadModel<'a, C: Clock<T = u64>> {
    last_keypress_time: Instant<C>,
    key_states: [KeyState; KEY_COUNT],
    actions: Vec<Action, KEY_COUNT>,
    keypad_time: Generic<u64>,
    key_mapper: KeyMapper<'a, C, KEY_COUNT>,
}

impl<'a, C: Clock<T = u64>> KeypadModel<'a, C> {
    pub fn new(clock: &'a C) -> Self {
        Self {
            last_keypress_time: clock.try_now().unwrap(),
            key_states: [KeyState::Up; 13],
            actions: Default::default(),
            keypad_time: Default::default(),
            key_mapper: KeyMapper::new(
                clock,
                [
                    Modifier::TapHold(
                        Action::Keyboard(Keyboard::Keypad7),
                        Action::App(AppAction::Bootloader),
                    ),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad8)),
                    Modifier::TapHold(
                        Action::Keyboard(Keyboard::Keypad9),
                        Action::App(AppAction::ShowTimings),
                    ),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad4)),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad5)),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad6)),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad1)),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad2)),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad3)),
                    Modifier::Single(Action::Keyboard(Keyboard::Keypad0)),
                    Modifier::Single(Action::Keyboard(Keyboard::KeypadDot)),
                    Modifier::Single(Action::Keyboard(Keyboard::KeypadEnter)),
                    Modifier::TapHold(
                        Action::Keyboard(Keyboard::KeypadNumLockAndClear),
                        Action::App(AppAction::ShowMenu),
                    ),
                ],
            ),
        }
    }

    pub fn set_actions(&mut self, actions: &[Action]) {
        self.actions.clear();
        self.actions.extend_from_slice(actions).unwrap();
    }
    pub fn actions(&self) -> &Vec<Action, 13> {
        &self.actions
    }

    pub(crate) fn set_keypad_time(&mut self, time: Generic<u64>) {
        self.keypad_time = time;
    }

    pub fn keypad_time(&self) -> Generic<u64> {
        self.keypad_time
    }

    pub fn key_states(&self) -> &'_ [KeyState; 13] {
        &self.key_states
    }
    pub fn set_key_states(&mut self, key_states: [KeyState; 13]) {
        self.key_states = key_states;
    }

    pub fn set_last_keypress_time(&mut self, now: &Instant<C>) {
        self.last_keypress_time = *now;
    }
    pub fn last_keypress_time(&self) -> &Instant<C> {
        &self.last_keypress_time
    }
    pub fn key_mapper(&mut self) -> &mut KeyMapper<'a, C, KEY_COUNT> {
        &mut self.key_mapper
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum KeyState {
    Up,
    Down,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Modifier {
    //NoAction,
    Single(Action),
    TapHold(Action, Action),
    //tap dance
}

pub enum ModifierState<C: Clock> {
    Single(Action),
    TapHold(Action, Action, TapHoldState<C>),
}

pub struct TapHoldState<C: Clock> {
    up: Instant<C>,
    down: Instant<C>,
}

impl<C: Clock> TapHoldState<C> {
    pub fn new() -> Self {
        Self {
            up: Instant::new(C::T::min_value()),
            down: Instant::new(C::T::min_value()),
        }
    }
}

impl<C: Clock<T = u64>> ModifierState<C> {
    pub(crate) fn map(&mut self, key_state: KeyState, now: Instant<C>) -> Option<Action> {
        const HOLD_DURATION: Milliseconds<u32> = Milliseconds(500u32);

        match self {
            ModifierState::Single(a) => match key_state {
                KeyState::Down => Some(*a),
                _ => None,
            },
            ModifierState::TapHold(a1, a2, ref mut s) => {
                let last_key_state = if s.down <= s.up {
                    KeyState::Up
                } else {
                    KeyState::Down
                };

                match (last_key_state, key_state) {
                    (KeyState::Up, KeyState::Down) => {
                        s.down = now;
                        log::info!("KeyDown");
                        None
                    }
                    (KeyState::Down, KeyState::Down) => {
                        //emit hold?
                        if Milliseconds::<u32>::try_from(now - s.down).unwrap() > HOLD_DURATION {
                            Some(*a2)
                        } else {
                            None
                        }
                    }
                    (KeyState::Down, KeyState::Up) => {
                        s.up = now;
                        log::info!(
                            "KeyUp {}",
                            Milliseconds::<u32>::try_from(now - s.down)
                                .unwrap()
                                .integer()
                        );
                        //emmit tap or hold?
                        if Milliseconds::<u32>::try_from(now - s.down).unwrap() <= HOLD_DURATION {
                            Some(*a1)
                        } else {
                            Some(*a2)
                        }
                    }
                    (KeyState::Up, KeyState::Up) => {
                        //emmit tap?
                        if Milliseconds::<u32>::try_from(s.up - s.down).unwrap() <= HOLD_DURATION
                            && Milliseconds::<u32>::try_from(s.up - s.down).unwrap()
                                >= Milliseconds::<u32>::try_from(now - s.up).unwrap()
                        {
                            Some(*a1)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }
}

pub struct KeyMapper<'a, C: Clock<T = u64>, const N: usize> {
    key_modifiers_states: [ModifierState<C>; N],
    clock: &'a C,
}

impl<'a, C: Clock<T = u64>, const N: usize> KeyMapper<'a, C, N> {
    pub fn new(clock: &'a C, modifiers: [Modifier; N]) -> Self {
        Self {
            clock,
            key_modifiers_states: modifiers.map(|m| match m {
                //Modifier::NoAction => ModifierState::None(m),
                Modifier::Single(a) => ModifierState::Single(a),
                Modifier::TapHold(a1, a2) => ModifierState::TapHold(a1, a2, TapHoldState::new()),
            }),
        }
    }

    pub(crate) fn map(&mut self, key_states: &[KeyState; N]) -> Vec<Action, N> {
        let now = self.clock.try_now().unwrap();

        key_states
            .iter()
            .enumerate()
            .filter_map(|(i, &k)| self.key_modifiers_states[i].map(k, now))
            .collect()
    }
}
