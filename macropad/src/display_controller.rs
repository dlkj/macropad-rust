use crate::{PeripheralsModel, UsbModel};

use crate::keypad_view::KeypadView;
use crate::models::application_model::{ApplicationModel, ApplicationView, Overlay};
use crate::models::keypad_model::KeypadModel;
use crate::models::DisplayModel;
use crate::overlays::TimingOverlayView;
use crate::time::Stopwatch;
use crate::views::screensaver_view::ScreensaverView;
use crate::views::status_view::StatusView;
use crate::views::text_view::TextView;
use embedded_time::duration::{Microseconds, Seconds};
use embedded_time::{Clock, Instant};
use sh1106::interface::DisplayInterface;

#[derive(Default)]
pub struct DisplayController {}

impl DisplayController {
    pub fn tick<DI: DisplayInterface, C: Clock<T = u64>>(
        &self,
        now: &Instant<C>,
        display_model: &mut DisplayModel<'_, DI, C>,
        macropad_model: &PeripheralsModel<'_, C>,
        key_model: &mut KeypadModel<'_, C>,
        app_model: &mut ApplicationModel,
        usb_model: &UsbModel<'_>,
    ) {
        if display_model.display_update_due() {
            let stopwatch = Stopwatch::new(macropad_model.clock()).unwrap();
            self.update_display(
                now,
                display_model,
                macropad_model,
                key_model,
                app_model,
                usb_model,
            );
            app_model.set_display_time(stopwatch.elaspsed().unwrap());
        }
    }

    fn update_display<DI: DisplayInterface, C: Clock<T = u64>>(
        &self,
        now: &Instant<C>,
        display_model: &mut DisplayModel<'_, DI, C>,
        macropad_model: &PeripheralsModel<'_, C>,
        key_model: &KeypadModel<'_, C>,
        app_model: &ApplicationModel,
        usb_model: &UsbModel<'_>,
    ) {
        display_model.display_clear();
        let f = display_model.frame_clounter_get_and_increment();

        if *key_model.last_keypress_time() + Seconds(60u32) < *now {
            display_model.set_contrast(0);
            display_model.display_draw(ScreensaverView::new(*now, usb_model.keyboard_leds()))
        } else {
            display_model.set_contrast(0xFF);
            match app_model.active_view() {
                ApplicationView::Log => {
                    display_model.display_draw(TextView::new(macropad_model.log_lines()));
                }
                ApplicationView::Status => {
                    display_model.display_draw(StatusView::new(
                        macropad_model.ticks_since_epoc(),
                        key_model.key_states(),
                        usb_model.keyboard_leds(),
                        usb_model.usb_state(),
                    ));
                }
                ApplicationView::Keypad => {
                    display_model.display_draw(KeypadView::new(
                        key_model.key_states(),
                        usb_model.keyboard_leds() & 1 > 0,
                    ));
                }
            }
        }

        match app_model.active_overlay() {
            Overlay::None => {}
            Overlay::ControllerTiming => {
                display_model.display_draw(TimingOverlayView::new(
                    Microseconds::try_from(app_model.display_time()).unwrap(),
                    Microseconds::try_from(key_model.keypad_time()).unwrap(),
                    f,
                ));
            }
        }
        display_model.display_flush();
    }
}
