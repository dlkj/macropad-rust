use crate::keypad_controller::{Action, AppAction, KeyState};
use crate::{DebouncedInputArray, Mutex, UsbState, LOGGER};
use atomic_polyfill::AtomicU8;
use core::cell::RefCell;
use core::default::Default;
use core::sync::atomic::Ordering;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::Drawable;
use embedded_hal::digital::v2::PinState;
use embedded_time::duration::{Generic, Microseconds, Milliseconds};
use embedded_time::timer::param::{Periodic, Running};
use embedded_time::{Clock, Timer};
use heapless::{String, Vec};
use sh1106::interface::DisplayInterface;
use sh1106::prelude::*;
use usb_device::device::UsbDeviceState;
use usbd_hid_devices::device::keyboard::NKROBootKeyboardInterface;
use usbd_hid_devices::page::Keyboard;
use usbd_hid_devices::UsbHidError;

const DISPLAY_UPDATE: Microseconds = Microseconds(16667);
const KEYPAD_UPDATE: Milliseconds = Milliseconds(20);

pub struct UsbModel<'a> {
    usb: &'a Mutex<RefCell<Option<UsbState>>>,
    leds: &'a AtomicU8,
}

impl<'a> UsbModel<'a> {
    pub(crate) fn new(usb: &'a Mutex<RefCell<Option<UsbState>>>, leds: &'a AtomicU8) -> Self {
        Self { usb, leds }
    }

    pub fn keyboard_leds(&self) -> u8 {
        self.leds.load(Ordering::SeqCst)
    }

    pub fn usb_state(&self) -> UsbDeviceState {
        cortex_m::interrupt::free(|cs| {
            if let Some((usb_device, _)) = self.usb.borrow(cs).borrow().as_ref() {
                usb_device.state()
            } else {
                UsbDeviceState::Default
            }
        })
    }

    pub(crate) fn write_keyboard_report<'b, K: IntoIterator<Item = &'b Keyboard>>(&self, keys: K) {
        cortex_m::interrupt::free(|cs| {
            if let Some((_, usb_hid)) = self.usb.borrow(cs).borrow().as_ref() {
                match usb_hid
                    .interface::<NKROBootKeyboardInterface<'_, _, _>, _>()
                    .write_report(keys)
                {
                    Err(UsbHidError::WouldBlock) => {}
                    Err(UsbHidError::Duplicate) => {}
                    Ok(_) => {}
                    Err(e) => {
                        panic!("Failed to write keyboard report: {:?}", e)
                    }
                }
            }
        })
    }
}

pub struct DisplayModel<'a, DI: DisplayInterface, C: Clock<T = u64>> {
    display: GraphicsMode<DI>,
    display_update_timer: Timer<'a, Periodic, Running, C, Microseconds>,
    frame_counter: u8,
}

impl<'a, DI: DisplayInterface, C: Clock<T = u64>> DisplayModel<'a, DI, C> {
    pub fn new(display: GraphicsMode<DI>, clock: &'a C) -> Self {
        let display_update_timer = clock
            .new_timer(DISPLAY_UPDATE)
            .into_periodic()
            .start()
            .unwrap();

        Self {
            display,
            display_update_timer,
            frame_counter: 0,
        }
    }

    pub fn display_draw<V: Drawable<Color = BinaryColor>>(&mut self, view: V) {
        view.draw(&mut self.display).unwrap();
    }

    pub fn display_clear(&mut self) {
        self.display.clear();
    }

    pub fn display_flush(&mut self) {
        self.display.flush().ok();
    }

    pub fn display_update_due(&mut self) -> bool {
        //todo fix issues with overflow after 1h10m
        self.display_update_timer.period_complete().unwrap()
    }

    pub fn frame_clounter_get_and_increment(&mut self) -> u8 {
        let (mut frame, _) = self.frame_counter.overflowing_add(1);

        if frame == 60 {
            frame = 0;
        }

        self.frame_counter = frame;
        frame
    }
}

pub struct ApplicationModel {
    active_view: ApplicationView,
    active_overlay: Overlay,
    display_time: Generic<u64>,
    last_actions: Vec<AppAction, 16>,
}

impl Default for ApplicationModel {
    fn default() -> Self {
        Self {
            active_view: ApplicationView::Keypad,
            active_overlay: Overlay::None,
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
}

pub struct KeypadModel {
    key_states: [KeyState; 13],
    actions: Vec<Action, 32>,
    keypad_time: Generic<u64>,
}

impl KeypadModel {
    pub fn actions(&self) -> &Vec<Action, 32> {
        &self.actions
    }
    pub fn set_actions<'a, I: IntoIterator<Item = &'a Action>>(&mut self, actions: I) {
        self.actions.clear();
        for action in actions.into_iter() {
            self.actions.push(*action).unwrap();
        }
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
}

impl Default for KeypadModel {
    fn default() -> Self {
        Self {
            key_states: [KeyState::Up; 13],
            actions: Default::default(),
            keypad_time: Default::default(),
        }
    }
}

pub struct PeripheralsModel<'a, C: Clock<T = u64>> {
    clock: &'a C,
    keys_update_timer: Timer<'a, Periodic, Running, C, Milliseconds>,
    keys: &'a DebouncedInputArray<13>,
}

impl<'a, C: Clock<T = u64>> PeripheralsModel<'a, C> {
    pub(crate) fn input_pin_values(&self) -> [PinState; 13] {
        self.keys.values()
    }

    pub fn new(clock: &'a C, keys: &'a DebouncedInputArray<13>) -> Self {
        let keypad_update_timer = clock
            .new_timer(KEYPAD_UPDATE)
            .into_periodic()
            .start()
            .unwrap();

        Self {
            keys_update_timer: keypad_update_timer,
            clock,
            keys,
        }
    }

    pub fn log_lines(&self) -> String<512> {
        LOGGER.log_buffer()
    }

    pub fn ticks_since_epoc(&self) -> u64 {
        self.clock
            .try_now()
            .unwrap()
            .duration_since_epoch()
            .integer()
    }

    pub fn keypad_update_due(&mut self) -> bool {
        //todo fix issues with overflow after 1h10m if using u32 clock
        self.keys_update_timer.period_complete().unwrap()
    }

    pub(crate) fn reboot_into_bootloader(&self) {
        //USB boot with pin 13 for usb activity
        adafruit_macropad::hal::rom_data::reset_to_usb_boot(0x1 << 13, 0x0);
    }

    pub fn clock(&self) -> &'a C {
        self.clock
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
