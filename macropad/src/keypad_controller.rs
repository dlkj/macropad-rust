use crate::models::{ApplicationModel, ApplicationView};
use crate::{PeripheralsModel, UsbModel};
use embedded_time::Clock;
use usbd_hid_devices::page::Keyboard;

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
            let keys = per_model.key_pin_values();

            app_model.set_key_values(keys.iter().enumerate().filter_map(|(i, &v)| {
                if v {
                    None
                } else {
                    Some(i)
                }
            }));

            if app_model.key_values().contains(&9) && app_model.key_values().contains(&12) {
                per_model.reboot_into_bootloader();
            }

            app_model.set_active_view(
                if app_model.key_values().contains(&9)
                    && app_model.key_values().contains(&10)
                    && app_model.key_values().contains(&11)
                {
                    ApplicationView::Log
                } else {
                    ApplicationView::Status
                },
            );

            if app_model.key_values().contains(&0) {
                usb_model.write_keyboard_report(&[Keyboard::LeftGUI]);
            } else {
                usb_model.write_keyboard_report(&[Keyboard::NoEventIndicated]);
            }
        }
    }
}
