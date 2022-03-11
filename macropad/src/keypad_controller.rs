use crate::models::ApplicationModel;
use crate::time::Stopwatch;
use crate::{PeripheralsModel, UsbModel};
use embedded_time::Clock;
use hash32::{Hash, Hasher};
use hash32_derive::Hash32;
use heapless::Vec;
use packed_struct::prelude::*;
use usbd_hid_devices::page::Keyboard;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash32)]
pub struct KeyPress(usize);

//todo hash32 support
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Action {
    None,
    Key(Keyboard),
}

impl Hash for Action {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        match self {
            Action::None => state.write(&[0]),
            Action::Key(k) => state.write(&[1, k.to_primitive()]),
        }
    }
}

#[derive(Default)]
pub struct KeypadController;

impl KeypadController {
    pub fn tick<C: Clock<T = u64>>(
        &self,
        per_model: &mut PeripheralsModel<'_, C>,
        app_model: &mut ApplicationModel,
        usb_model: &mut UsbModel<'_>,
    ) {
        if per_model.keypad_update_due() {
            let stopwatch = Stopwatch::new(per_model.clock()).unwrap();

            self.update_key_presses(app_model, per_model.input_pin_values());

            let actions = self.apply_key_map(app_model);

            // if app_model.key_values().contains(&KeyPress(9))
            //     && app_model.key_values().contains(&KeyPress(12))
            // {
            //     per_model.reboot_into_bootloader();
            // }

            // app_model.set_active_view(
            //     if app_model.key_values().contains(&KeyPress(9))
            //         && app_model.key_values().contains(&KeyPress(10))
            //         && app_model.key_values().contains(&KeyPress(11))
            //     {
            //         ApplicationView::Log
            //     } else {
            //         ApplicationView::Status
            //     },
            // );
            app_model.set_actions(&actions);
            usb_model.write_keyboard_report(actions.iter().filter_map(|a| match a {
                Action::None => None,
                Action::Key(k) => Some(k),
            }));
            app_model.set_keypad_time(stopwatch.elaspsed().unwrap());
        }
    }

    fn update_key_presses(&self, app_model: &mut ApplicationModel, keys: [bool; 13]) {
        app_model.set_key_presses(keys.iter().enumerate().filter_map(|(i, &v)| {
            if v {
                None
            } else {
                Some(KeyPress(i))
            }
        }));
    }
    fn apply_key_map(&self, app_model: &mut ApplicationModel) -> Vec<Action, 16> {
        let keys = app_model.key_presses();

        keys.iter()
            .map(|&k| match k {
                KeyPress(0) => Action::Key(Keyboard::Keypad7),
                KeyPress(1) => Action::Key(Keyboard::Keypad8),
                KeyPress(2) => Action::Key(Keyboard::Keypad9),
                KeyPress(3) => Action::Key(Keyboard::Keypad4),
                KeyPress(4) => Action::Key(Keyboard::Keypad5),
                KeyPress(5) => Action::Key(Keyboard::Keypad6),
                KeyPress(6) => Action::Key(Keyboard::Keypad1),
                KeyPress(7) => Action::Key(Keyboard::Keypad2),
                KeyPress(8) => Action::Key(Keyboard::Keypad3),
                KeyPress(9) => Action::Key(Keyboard::Keypad0),
                KeyPress(10) => Action::Key(Keyboard::KeypadDot),
                KeyPress(11) => Action::Key(Keyboard::KeypadEnter),
                KeyPress(12) => Action::Key(Keyboard::KeypadNumLockAndClear),

                KeyPress(_) => Action::None,
            })
            .collect()
    }
}
