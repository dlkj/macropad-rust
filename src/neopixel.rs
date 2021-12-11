use smart_leds::{brightness, SmartLedsWrite, RGB8};

pub struct Neopixels<S> {
    ws: S,
    n: u16,
}

impl<S, E> Neopixels<S>
where
    S: SmartLedsWrite<Error = E>,
{
    pub fn new(ws: S) -> Neopixels<S> {
        Neopixels { ws, n: 0 }
    }

    pub fn update(&mut self) -> Result<(), E>
    where
        S::Color: From<RGB8>,
        S: SmartLedsWrite,
    {
        self.ws
            .write(brightness(itertools::repeat_n(wheel(self.n), 12), 32))?;
        self.n = (self.n + 1) % 768;

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
