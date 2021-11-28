use embedded_hal::timer::CountDown;
use embedded_hal::timer::Periodic;
use embedded_time::duration::*;
use rp2040_hal::gpio::Function;
use rp2040_hal::gpio::FunctionConfig;
use rp2040_hal::gpio::PinId;
use rp2040_hal::gpio::ValidPinMode;
use rp2040_hal::pio::PIOExt;
use rp2040_hal::pio::StateMachineIndex;
use smart_leds::{brightness, SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;

pub struct Neopixels<P, SM, C, T, I>
where
    I: PinId,
    C: CountDown<Time = T> + Periodic,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    SM: StateMachineIndex,
    T: From<Microseconds>,
{
    ws: Ws2812<P, SM, C, I>,
    countdown: C,
    n: u16,
}

impl<I: PinId, C: CountDown + Periodic, T, P: PIOExt + FunctionConfig, SM: StateMachineIndex>
    Neopixels<P, SM, C, T, I>
where
    Function<P>: ValidPinMode<I>,
    C: CountDown<Time = T>,
    T: From<Microseconds>,
{
    pub fn new<CP>(
        ws: Ws2812<P, SM, C, I>,
        mut countdown: C,
        period: CP,
    ) -> Neopixels<P, SM, C, T, I>
    where
        CP: Into<T>,
    {
        let p = period.into();
        countdown.start(p);
        Neopixels {
            ws,
            countdown,
            n: 0,
        }
    }

    pub fn update(&mut self) -> Result<(), ()> {
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
