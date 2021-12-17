use smart_leds::{brightness, gamma, SmartLedsWrite, RGB8};

const WHEEL_STEPS: u16 = u8::MAX as u16 * 3;

pub struct Neopixels<S, const LEN: usize> {
    ws: S,
    n: u16,
}

impl<S, E, const LEN: usize> Neopixels<S, LEN>
where
    S: SmartLedsWrite<Error = E>,
{
    pub fn new(ws: S) -> Neopixels<S, LEN> {
        Neopixels { ws, n: 0 }
    }

    pub fn update(&mut self, keys: &[usize]) -> Result<(), E>
    where
        S::Color: From<RGB8>,
        S: SmartLedsWrite,
    {
        const BRIGHTNESS: u8 = 128;
        let led_steps: u16 = WHEEL_STEPS / LEN as u16;

        let key_colours = (0..LEN).map(|i| {
            if keys.contains(&i) {
                smart_leds::colors::WHITE
            } else {
                wheel((self.n + i as u16 * led_steps) % WHEEL_STEPS)
            }
        });

        self.ws.write(brightness(gamma(key_colours), BRIGHTNESS))?;
        self.n = (self.n + 1) % WHEEL_STEPS;

        Ok(())
    }
}

/// Convert a number from `0..=255*3` to an RGB color triplet.
fn wheel(mut wheel_pos: u16) -> RGB8 {
    if wheel_pos < u8::MAX as u16 {
        // No green in this sector - red and blue only
        (u8::MAX - wheel_pos as u8, 0, wheel_pos as u8).into()
    } else if wheel_pos < u8::MAX as u16 * 2 {
        // No red in this sector - green and blue only
        wheel_pos -= u8::MAX as u16;
        (0, wheel_pos as u8, u8::MAX - 1 - wheel_pos as u8).into()
    } else {
        // No blue in this sector - red and green only
        wheel_pos -= u8::MAX as u16 * 2;
        (wheel_pos as u8, u8::MAX - 1 - wheel_pos as u8, 0).into()
    }
}
