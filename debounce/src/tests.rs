use super::DebouncedPin;
use embedded_hal::digital::v2::InputPin;
use failure::Fail;

#[derive(Debug, Fail)]
#[fail(display = "An error occurred")]
struct TestPinError;

struct TestInputPin {
    value: bool,
}
impl embedded_hal::digital::v2::InputPin for TestInputPin {
    type Error = TestPinError;
    fn is_high(&self) -> core::result::Result<bool, Self::Error> {
        Ok(self.value)
    }
    fn is_low(&self) -> core::result::Result<bool, Self::Error> {
        Ok(!self.value)
    }
}

impl TestInputPin {
    fn new(value: bool) -> TestInputPin {
        TestInputPin { value }
    }
    fn set_value(&mut self, value: bool) {
        self.value = value;
    }
}

#[test]
fn is_low_if_starts_low_unbounced() {
    let test_pin = TestInputPin::new(false);
    let mut debouncer = DebouncedPin::new(test_pin, false);
    debouncer.update().unwrap();

    assert!(debouncer.is_low().unwrap());
}

#[test]
fn is_high_if_starts_high_unbounced() {
    let test_pin = TestInputPin::new(true);
    let mut debouncer = DebouncedPin::new(test_pin, true);
    debouncer.update().unwrap();

    assert!(debouncer.is_high().unwrap());
}

#[test]
fn is_low_if_starts_high_bounced() {
    let test_pin = TestInputPin::new(true);
    let mut debouncer = DebouncedPin::new(test_pin, false);
    debouncer.update().unwrap();

    assert!(debouncer.is_low().unwrap());
}

#[test]
fn is_heigh_if_starts_low_bounced() {
    let test_pin = TestInputPin::new(false);
    let mut debouncer = DebouncedPin::new(test_pin, true);
    debouncer.update().unwrap();

    assert!(debouncer.is_high().unwrap());
}

#[test]
fn change_after_5_consecutive_reads_high() {
    let test_pin = TestInputPin::new(true);
    let mut debouncer = DebouncedPin::new(test_pin, false);
    for _ in 0..4 {
        debouncer.update().unwrap();
        assert!(debouncer.is_low().unwrap());
    }

    debouncer.update().unwrap();
    assert!(debouncer.is_high().unwrap());
}

#[test]
fn change_after_5_consecutive_reads_low() {
    let test_pin = TestInputPin::new(false);
    let mut debouncer = DebouncedPin::new(test_pin, true);
    for _ in 0..4 {
        debouncer.update().unwrap();
        assert!(debouncer.is_high().unwrap());
    }

    debouncer.update().unwrap();
    assert!(debouncer.is_low().unwrap());
}
#[test]
fn change_high_after_5_consecutive_afterbouncing_reads() {
    let test_pin = TestInputPin::new(false);
    let mut debouncer = DebouncedPin::new(test_pin, false);
    for i in 0..14 {
        debouncer.pin.set_value(i % 2 == 0);
        debouncer.update().unwrap();
        assert!(debouncer.is_low().unwrap());
    }
    debouncer.pin.set_value(true);
    for _ in 0..4 {
        debouncer.update().unwrap();
        assert!(debouncer.is_low().unwrap());
    }

    debouncer.update().unwrap();
    assert!(debouncer.is_high().unwrap());
}

#[test]
fn change_low_after_5_consecutive_afterbouncing_reads() {
    let test_pin = TestInputPin::new(true);
    let mut debouncer = DebouncedPin::new(test_pin, true);
    for i in 0..15 {
        debouncer.pin.set_value(i % 2 == 0);
        debouncer.update().unwrap();
        assert!(debouncer.is_high().unwrap());
    }
    debouncer.pin.set_value(false);
    for _ in 0..4 {
        debouncer.update().unwrap();
        assert!(debouncer.is_high().unwrap());
    }

    debouncer.update().unwrap();
    assert!(debouncer.is_low().unwrap());
}
