use crate::MacropadModel;

use crate::macropad_model::DisplayMode;
use crate::number_view::NumberView;
use crate::text_view::TextView;
use embedded_time::Clock;
use sh1106::interface::DisplayInterface;

pub struct DisplayController<'a, DI: DisplayInterface, C: Clock<T = u32>> {
    model: MacropadModel<'a, DI, C>,
}

impl<'a, DI: DisplayInterface, C: Clock<T = u32>> DisplayController<'a, DI, C> {
    pub fn tick(&mut self) {
        if self.model.display_update_due() {
            self.update_display()
        }
    }

    fn update_display(&mut self) {
        match self.model.display_mode() {
            DisplayMode::Log => {
                self.model.display_draw(TextView::new(self.model.log()));
            }
            DisplayMode::Time => {
                self.model
                    .display_draw(NumberView::new(self.model.ticks_since_epoc()));
            }
        }
    }

    pub fn new(model: MacropadModel<'a, DI, C>) -> Self {
        Self { model }
    }
}
