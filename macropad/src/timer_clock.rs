use crate::hal;
use embedded_time::clock::Error;
use embedded_time::duration::Fraction;
use embedded_time::{Clock, Instant};

pub struct TimerClock {
    timer: hal::Timer,
}

impl TimerClock {
    pub fn new(timer: hal::Timer) -> Self {
        Self { timer }
    }
}

impl Clock for TimerClock {
    type T = u64;
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        Ok(Instant::new(self.timer.get_counter()))
    }
}

unsafe impl Sync for TimerClock {
    //safety - reading the timer counter is threadsafe - only consists of atomic reads
}
