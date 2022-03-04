use core::cell::RefCell;
use core::fmt::Write;
use core::writeln;

use heapless::String;
use log::{Level, Metadata, Record};

use crate::Mutex;

const BUFFER_SIZE: usize = 512;
const TRUNCATE_SIZE: usize = 32 * 9;

pub struct Logger {
    buffer: Mutex<RefCell<String<BUFFER_SIZE>>>,
}

impl log::Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    //todo investigated using ufmt for logging
    fn log(&self, record: &Record) {
        let mut log_str = String::<BUFFER_SIZE>::new();
        writeln!(
            &mut log_str,
            "{} {} {}",
            Self::level_str(record.level()),
            record
                .target()
                .split("::")
                .last()
                .unwrap_or_else(|| record.target()),
            record.args()
        )
        .ok();

        cortex_m::interrupt::free(|cs| {
            let buffer_ref = self.buffer.borrow(cs);
            let mut buffer = buffer_ref.borrow_mut();

            if buffer.len() + log_str.len() > buffer.capacity() {
                if log_str.len() >= TRUNCATE_SIZE {
                    let s = &log_str.as_str()[log_str.len() - TRUNCATE_SIZE..];
                    buffer_ref.replace(String::from(s));
                } else {
                    let s = &buffer.as_str()[buffer.len() + log_str.len() - TRUNCATE_SIZE..];
                    buffer_ref.replace(String::from(s));
                    buffer_ref.borrow_mut().push_str(&log_str).unwrap();
                }
            } else {
                buffer.push_str(&log_str).unwrap()
            }
        });
    }

    fn flush(&self) {}
}

impl Default for Logger {
    fn default() -> Self {
        Self::default()
    }
}

impl Logger {
    pub const fn default() -> Self {
        Self {
            buffer: Mutex::new(RefCell::new(String::new())),
        }
    }

    fn level_str(level: Level) -> &'static str {
        match level {
            Level::Error => "E",
            Level::Warn => "W",
            Level::Info => "I",
            Level::Debug => "D",
            Level::Trace => "T",
        }
    }
}
