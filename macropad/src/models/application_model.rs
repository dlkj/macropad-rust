use crate::keypad_controller::AppAction;
use embedded_time::duration::Generic;
use heapless::Vec;

pub struct ApplicationModel {
    active_view: ApplicationView,
    active_overlay: Overlay,
    display_time: Generic<u64>,
    last_actions: Vec<AppAction, 16>,
    menu: MenuState,
}

impl Default for ApplicationModel {
    fn default() -> Self {
        Self {
            active_view: ApplicationView::Keypad,
            active_overlay: Overlay::None,
            menu: MenuState::Closed,
            display_time: Default::default(),
            last_actions: Default::default(),
        }
    }
}

impl ApplicationModel {
    pub fn active_view(&self) -> ApplicationView {
        self.active_view
    }
    pub fn set_active_view(&mut self, active_view: ApplicationView) {
        self.active_view = active_view;
    }
    pub fn active_overlay(&self) -> Overlay {
        self.active_overlay
    }
    pub fn set_active_overlay(&mut self, active_overlay: Overlay) {
        self.active_overlay = active_overlay;
    }
    pub(crate) fn set_display_time(&mut self, time: Generic<u64>) {
        self.display_time = time;
    }
    pub fn display_time(&self) -> Generic<u64> {
        self.display_time
    }

    pub fn last_actions(&self) -> &Vec<AppAction, 16> {
        &self.last_actions
    }

    pub fn set_last_actions<'a, I: IntoIterator<Item = &'a AppAction>>(&mut self, last_actions: I) {
        self.last_actions.clear();
        for action in last_actions.into_iter() {
            self.last_actions.push(*action).unwrap();
        }
    }

    pub fn menu(&self) -> &MenuState {
        &self.menu
    }

    pub fn set_menu(&mut self, menu: MenuState) {
        self.menu = menu;
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ApplicationView {
    Log,
    Status,
    Keypad,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Overlay {
    None,
    ControllerTiming,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum MenuState {
    Closed,
    Open(ApplicationView),
}
