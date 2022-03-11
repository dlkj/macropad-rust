use crate::hal;
use core::ops::Sub;
use embedded_time::clock::Error;
use embedded_time::duration::{Fraction, Generic};
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

pub struct Stopwatch<'a, C: Clock> {
    clock: &'a C,
    start: Instant<C>,
}

impl<'a, C: Clock> Stopwatch<'a, C> {
    pub fn new(clock: &'a C) -> Result<Self, Error> {
        Ok(Self {
            clock,
            start: clock.try_now()?,
        })
    }

    pub fn elaspsed(&self) -> Result<Generic<C::T>, Error> {
        let now = self.clock.try_now()?;
        Ok(now.sub(self.start))
    }
}
