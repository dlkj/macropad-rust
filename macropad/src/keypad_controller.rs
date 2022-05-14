use crate::models::application_model::{ApplicationModel, MenuState, Overlay};
use crate::models::keypad_model::{KeyState, KeypadModel};
use crate::time::Stopwatch;
use crate::{PeripheralsModel, UsbModel};
use embedded_hal::digital::v2::PinState;
use embedded_time::{Clock, Instant};
use usbd_human_interface_device::page::Keyboard;

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
    ShowTimings,
    ShowMenu,
}

// #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
// pub enum MouseAction {
//     Button(u8),
//     X(i8),
//     Y(i8),
//     Wheel(i8),
//     Pan(i8),
// }

pub struct KeypadController {}

impl KeypadController {
    pub fn new() -> Self {
        Self {}
    }
    pub fn tick<C: Clock<T = u64>>(
        &mut self,
        now: &Instant<C>,
        per_model: &mut PeripheralsModel<'_, C>,
        key_model: &mut KeypadModel<'_, C>,
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
            if key_states.iter().any(|&k| k == KeyState::Down) {
                key_model.set_last_keypress_time(now);
            }

            let actions = key_model.key_mapper().map(&key_states);

            key_model.set_actions(&actions);

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
                    AppAction::ShowMenu => {
                        app_model.set_menu(MenuState::Open(app_model.active_view()))
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
