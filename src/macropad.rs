use crate::KeyboardReport;
use crate::USB_KEYBOARD;
use embedded_hal::digital::v2::InputPin;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::timer::CountDown;
use embedded_hal::timer::Periodic;
use embedded_time::duration::*;
use log::info;

pub struct Macropad<BP, LP, C>
where
    BP: InputPin,
    LP: OutputPin,
    C: CountDown + Periodic,
{
    count: u8,
    pressed: bool,
    button_pin: BP,
    led_pin: LP,
    countdown: C,
}

impl<BP: InputPin, LP: OutputPin, C: CountDown + Periodic> Macropad<BP, LP, C> {
    pub fn new<T>(button_pin: BP, led_pin: LP, mut countdown: C) -> Macropad<BP, LP, C>
    where
        BP: InputPin,
        LP: OutputPin,
        C: CountDown<Time = T>,
        T: From<Milliseconds>,
    {
        countdown.start(10.milliseconds());

        Macropad {
            count: 0,
            pressed: false,
            button_pin,
            led_pin,
            countdown,
        }
    }

    pub fn update<PinE>(&mut self) -> Result<(), PinE>
    where
        BP: InputPin<Error = PinE>,
        LP: OutputPin<Error = PinE>,
    {
        if self.button_pin.is_low()? && !self.pressed {
            self.led_pin.set_high()?;
            self.pressed = true;

            info!("Button pressed {}", self.count);
            self.count = self.count.wrapping_add(1);
        } else if self.button_pin.is_high()? {
            self.led_pin.set_low()?;
            self.pressed = false;
        }

        match self.countdown.wait() {
            Ok(_) => cortex_m::interrupt::free(|cs| {
                let mut keyboard_ref = USB_KEYBOARD.borrow(cs).borrow_mut();
                let keyboard = keyboard_ref.as_mut().unwrap();

                if self.button_pin.is_low()? {
                    let _ = keyboard.push_input(&KeyboardReport {
                        modifier: 0,
                        leds: 0,
                        reserved: 0,
                        keycodes: [0x04, 0, 0, 0, 0, 0], //a
                    });
                } else {
                    let _ = keyboard.push_input(&KeyboardReport {
                        modifier: 0,
                        leds: 0,
                        reserved: 0,
                        keycodes: [0, 0, 0, 0, 0, 0],
                    });
                }

                Ok(())
            }),
            Err(_) => Ok(()),
        }
    }
}
