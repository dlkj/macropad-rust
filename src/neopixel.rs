use embedded_hal::timer::CountDown;
use embedded_hal::timer::Periodic;
use embedded_time::duration::*;
use smart_leds::{brightness, SmartLedsWrite, RGB8};

pub struct Neopixels<S, C> {
    ws: S,
    countdown: C,
    n: u16,
}

impl<S, E, C, T> Neopixels<S, C>
where
    S: SmartLedsWrite<Error = E>,
    C: CountDown<Time = T> + Periodic,
    T: From<Microseconds>,
{
    pub fn new<P>(ws: S, mut countdown: C, period: P) -> Neopixels<S, C>
    where
        P: Into<T>,
    {
        let p = period.into();
        countdown.start(p);
        Neopixels {
            ws,
            countdown,
            n: 0,
        }
    }

    pub fn update(&mut self) -> Result<(), E>
    where
        S::Color: From<RGB8>,
        S: SmartLedsWrite,
    {
        match self.countdown.wait() {
            Ok(_) => {
                self.ws
                    .write(brightness(itertools::repeat_n(wheel(self.n), 12), 32))?;
                self.n = (self.n + 1) % 768;
            }
            Err(_) => {}
        }
        Ok(())
    }
}

/// Convert a number from `0..=255*3` to an RGB color triplet.
fn wheel(mut wheel_pos: u16) -> RGB8 {
    if wheel_pos < 256 {
        // No green in this sector - red and blue only
        (255 - wheel_pos as u8, 0, wheel_pos as u8).into()
    } else if wheel_pos < 512 {
        // No red in this sector - green and blue only
        wheel_pos -= 256;
        (0, wheel_pos as u8, 255 - wheel_pos as u8).into()
    } else {
        // No blue in this sector - red and green only
        wheel_pos -= 512;
        (wheel_pos as u8, 255 - wheel_pos as u8, 0).into()
    }
}
