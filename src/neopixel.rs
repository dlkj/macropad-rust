use embedded_hal::timer::CountDown;

use embedded_time::duration::*;
use rp2040_hal::gpio::Function;
use rp2040_hal::gpio::FunctionConfig;
use rp2040_hal::gpio::PinId;
use rp2040_hal::gpio::ValidPinMode;
use rp2040_hal::pio::PIOExt;
use rp2040_hal::pio::StateMachineIndex;
use smart_leds::{brightness, SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;

pub struct Neopixels<P, SM, C, I>
where
    I: PinId,
    C: CountDown,
    P: PIOExt + FunctionConfig,
    Function<P>: ValidPinMode<I>,
    SM: StateMachineIndex,
{
    ws: Ws2812<P, SM, C, I>,
    countdown: C,
    n: u16,
    period: Microseconds<u64>,
}

impl<I: PinId, C: CountDown, P: PIOExt + FunctionConfig, SM: StateMachineIndex>
    Neopixels<P, SM, C, I>
where
    rp2040_hal::gpio::Function<P>: rp2040_hal::gpio::ValidPinMode<I>,
    C: CountDown<Time = Microseconds<u64>>,
{
    pub fn new<Period>(
        ws: Ws2812<P, SM, C, I>,
        mut countdown: C,
        period: Period,
    ) -> Neopixels<P, SM, C, I>
    where
        Period: Into<Microseconds<u64>>,
    {
        let p = period.into();
        countdown.start(p);
        Neopixels {
            ws,
            countdown,
            n: 0,
            period: p,
        }
    }

    pub fn update(&mut self) {
        match self.countdown.wait() {
            Ok(_) => {
                self.ws
                    .write(brightness(itertools::repeat_n(wheel(self.n), 12), 32))
                    .unwrap();
                self.n = (self.n + 1) % 768;
                self.countdown.start(self.period);
            }
            Err(_) => {}
        }
    }
}

/// Convert a number from `0..=255` to an RGB color triplet.
///
/// The colours are a transition from red, to green, to blue and back to red.
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
