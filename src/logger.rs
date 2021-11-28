pub struct MacropadLogger;

use crate::USB_SERIAL;
use core::{fmt, fmt::Write};
use log::{Level, Metadata, Record};

impl fmt::Write for MacropadLogger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        cortex_m::interrupt::free(|cs| {
            let mut serial_ref = USB_SERIAL.borrow(cs).borrow_mut();
            if let Some(serial) = serial_ref.as_mut() {
                serial.write(s.as_bytes()).map_or_else(
                    |_error| fmt::Result::Err(fmt::Error),
                    |_c| fmt::Result::Ok(()),
                )
            } else {
                fmt::Result::Ok(())
            }
        })
    }
}

impl log::Log for MacropadLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut writer = MacropadLogger;
            //serial port is probabbly not connected, better to swallow failures than panic
            let _ = write!(&mut writer, "{} - {}\r\n", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
