use crate::{PeripheralsModel, UsbModel};

use crate::models::{ApplicationModel, ApplicationView, DisplayModel};
use crate::status_view::StatusView;
use crate::text_view::TextView;
use embedded_time::Clock;
use sh1106::interface::DisplayInterface;

#[derive(Default)]
pub struct DisplayController {}

impl DisplayController {
    pub fn tick<DI: DisplayInterface, C: Clock<T = u64>>(
        &self,
        display_model: &mut DisplayModel<'_, DI, C>,
        macropad_model: &PeripheralsModel<'_, C>,
        app_model: &ApplicationModel,
        usb_model: &UsbModel<'_>,
    ) {
        if display_model.display_update_due() {
            self.update_display(display_model, macropad_model, app_model, usb_model)
        }
    }

    fn update_display<DI: DisplayInterface, C: Clock<T = u64>>(
        &self,
        display_model: &mut DisplayModel<'_, DI, C>,
        macropad_model: &PeripheralsModel<'_, C>,
        app_model: &ApplicationModel,
        usb_model: &UsbModel<'_>,
    ) {
        match app_model.active_view() {
            ApplicationView::Log => {
                display_model.display_draw(TextView::new(macropad_model.log_lines()));
            }
            ApplicationView::Status => {
                display_model.display_draw(StatusView::new(
                    macropad_model.ticks_since_epoc(),
                    app_model.key_values(),
                    usb_model.keyboard_leds(),
                    usb_model.usb_state(),
                ));
            }
        }
    }
}
