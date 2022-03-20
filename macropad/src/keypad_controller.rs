use crate::models::{ApplicationModel, ApplicationView, KeypadModel, Overlay};
use crate::time::Stopwatch;
use crate::{PeripheralsModel, UsbModel};
use embedded_hal::digital::v2::PinState;
use embedded_time::duration::Milliseconds;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::{Clock, Instant};
use hash32_derive::Hash32;
use heapless::Vec;
use num_traits::Bounded;
use usbd_hid_devices::page::Keyboard;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum KeyState {
    Up,
    Down,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash32)]
pub struct Chord(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Action {
    App(AppAction),
    Keyboard(Keyboard),
    //Mouse(MouseAction),
    //Consumer(Consumer),
    //Macro
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum AppAction {
    Bootloader,
    ChangeView,
    ShowTimings,
}

// #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
// pub enum MouseAction {
//     Button(u8),
//     X(i8),
//     Y(i8),
//     Wheel(i8),
//     Pan(i8),
// }

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

pub struct KeypadController<'a, C: Clock<T = u64>> {
    key_mapper: KeyMapper<'a, C, 13>,
}

impl<'a, C: Clock<T = u64>> KeypadController<'a, C> {
    pub fn new(clock: &'a C) -> Self {
        Self {
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
                        Action::App(AppAction::ChangeView),
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
                        Action::App(AppAction::ShowTimings),
                    ),
                ],
            ),
        }
    }
    pub fn tick(
        &mut self,
        per_model: &mut PeripheralsModel<'_, C>,
        key_model: &mut KeypadModel,
        usb_model: &mut UsbModel<'_>,
        app_model: &mut ApplicationModel,
    ) {
        if per_model.keypad_update_due() {
            let stopwatch = Stopwatch::new(per_model.clock()).unwrap();

            //pin states to key presses
            let keys = per_model.input_pin_values();

            let key_states = keys.map(|p| match p {
                PinState::High => KeyState::Up,
                PinState::Low => KeyState::Down,
            });

            key_model.set_key_states(key_states);

            //map key presses to actions
            let actions = self.key_mapper.map(key_model.key_states());

            key_model.set_actions(&actions);
            let actions = key_model.actions();

            //process modifiers

            //macro actions

            //application actions
            let app_actions = actions.iter().filter_map(|a| match a {
                Action::App(app) => Some(app),
                _ => None,
            });

            for a in app_actions {
                if app_model.last_actions().contains(a) {
                    continue;
                }
                match a {
                    AppAction::Bootloader => {
                        per_model.reboot_into_bootloader();
                    }
                    AppAction::ChangeView => {
                        let next_view = match app_model.active_view() {
                            ApplicationView::Log => ApplicationView::Status,
                            ApplicationView::Status => ApplicationView::Keypad,
                            ApplicationView::Keypad => ApplicationView::Log,
                        };
                        app_model.set_active_view(next_view);
                    }
                    AppAction::ShowTimings => {
                        let next_overlay = match app_model.active_overlay() {
                            Overlay::None => Overlay::ControllerTiming,
                            Overlay::ControllerTiming => Overlay::None,
                        };
                        app_model.set_active_overlay(next_overlay);
                    }
                }
            }

            app_model.set_last_actions(actions.iter().filter_map(|a| match a {
                Action::App(app) => Some(app),
                _ => None,
            }));

            //keyboard actions
            usb_model.write_keyboard_report(actions.iter().filter_map(|a| match a {
                Action::Keyboard(k) => Some(k),
                _ => None,
            }));

            //mouse actions

            //consumer actions

            //macro action cleanup

            key_model.set_keypad_time(stopwatch.elaspsed().unwrap());
        }
    }
}
