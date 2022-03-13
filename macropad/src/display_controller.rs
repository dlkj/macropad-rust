use crate::{PeripheralsModel, UsbModel};

use crate::keypad_view::KeypadView;
use crate::models::{ApplicationModel, ApplicationView, DisplayModel, Overlay};
use crate::overlays::TimingOverlayView;
use crate::status_view::StatusView;
use crate::text_view::TextView;
use crate::time::Stopwatch;
use embedded_time::duration::Microseconds;
use embedded_time::Clock;
use sh1106::interface::DisplayInterface;

#[derive(Default)]
pub struct DisplayController {}

impl DisplayController {
    pub fn tick<DI: DisplayInterface, C: Clock<T = u64>>(
        &self,
        display_model: &mut DisplayModel<'_, DI, C>,
        macropad_model: &PeripheralsModel<'_, C>,
        app_model: &mut ApplicationModel,
        usb_model: &UsbModel<'_>,
    ) {
        if display_model.display_update_due() {
            let stopwatch = Stopwatch::new(macropad_model.clock()).unwrap();
            self.update_display(display_model, macropad_model, app_model, usb_model);
            app_model.set_display_time(stopwatch.elaspsed().unwrap());
        }
    }

    fn update_display<DI: DisplayInterface, C: Clock<T = u64>>(
        &self,
        display_model: &mut DisplayModel<'_, DI, C>,
        macropad_model: &PeripheralsModel<'_, C>,
        app_model: &ApplicationModel,
        usb_model: &UsbModel<'_>,
    ) {
        display_model.display_clear();
        let f = display_model.frame_clounter_get_and_increment();

        match app_model.active_view() {
            ApplicationView::Log => {
                display_model.display_draw(TextView::new(macropad_model.log_lines()));
            }
            ApplicationView::Status => {
                display_model.display_draw(StatusView::new(
                    macropad_model.ticks_since_epoc(),
                    app_model.key_presses(),
                    usb_model.keyboard_leds(),
                    usb_model.usb_state(),
                ));
            }
            ApplicationView::Keypad => {
                display_model.display_draw(KeypadView::new(
                    app_model.actions(),
                    usb_model.keyboard_leds() & 1 > 0,
                ));
            }
        }

        match app_model.active_overlay() {
            Overlay::None => {}
            Overlay::ControllerTiming => {
                display_model.display_draw(TimingOverlayView::new(
                    Microseconds::try_from(app_model.display_time()).unwrap(),
                    Microseconds::try_from(app_model.keypad_time()).unwrap(),
                    f,
                ));
            }
        }
        display_model.display_flush();
    }
}
