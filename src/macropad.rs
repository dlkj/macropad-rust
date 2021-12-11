use embedded_hal::digital::v2::InputPin;

pub struct Macropad<K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11, K12> {
    key1: K1,
    key2: K2,
    key3: K3,
    key4: K4,
    key5: K5,
    key6: K6,
    key7: K7,
    key8: K8,
    key9: K9,
    key10: K10,
    key11: K11,
    key12: K12,
}

impl<K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11, K12, PinE>
    Macropad<K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11, K12>
where
    K1: InputPin<Error = PinE>,
    K2: InputPin<Error = PinE>,
    K3: InputPin<Error = PinE>,
    K4: InputPin<Error = PinE>,
    K5: InputPin<Error = PinE>,
    K6: InputPin<Error = PinE>,
    K7: InputPin<Error = PinE>,
    K8: InputPin<Error = PinE>,
    K9: InputPin<Error = PinE>,
    K10: InputPin<Error = PinE>,
    K11: InputPin<Error = PinE>,
    K12: InputPin<Error = PinE>,
    PinE: core::fmt::Debug,
{
    pub fn new(
        key1: K1,
        key2: K2,
        key3: K3,
        key4: K4,
        key5: K5,
        key6: K6,
        key7: K7,
        key8: K8,
        key9: K9,
        key10: K10,
        key11: K11,
        key12: K12,
    ) -> Macropad<K1, K2, K3, K4, K5, K6, K7, K8, K9, K10, K11, K12> {
        Macropad {
            key1,
            key2,
            key3,
            key4,
            key5,
            key6,
            key7,
            key8,
            key9,
            key10,
            key11,
            key12,
        }
    }

    pub fn get_keycodes(&self) -> [u8; 6] {
        let mut keycodes: [u8; 6] = [0, 0, 0, 0, 0, 0];

        let mut k_it = keycodes.iter_mut();

        k_it.next()
            .and_then(|k| {
                if self.key1.is_low().unwrap() {
                    *k = 0x5f; //Numpad 7
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key2.is_low().unwrap() {
                    *k = 0x60; //Numpad 8
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key3.is_low().unwrap() {
                    *k = 0x61; //Numpad 9
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key4.is_low().unwrap() {
                    *k = 0x5c; //Numpad 4
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key5.is_low().unwrap() {
                    *k = 0x5d; //Numpad 5
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key6.is_low().unwrap() {
                    *k = 0x5e; //Numpad 6
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key7.is_low().unwrap() {
                    *k = 0x59; //Numpad 1
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key8.is_low().unwrap() {
                    *k = 0x5a; //Numpad 2
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key9.is_low().unwrap() {
                    *k = 0x5b; //Numpad 3
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key10.is_low().unwrap() {
                    *k = 0x62; //Numpad 0
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key11.is_low().unwrap() {
                    *k = 0x63; //Numpad .
                    k_it.next()
                } else {
                    Some(k)
                }
            })
            .and_then(|k| {
                if self.key12.is_low().unwrap() {
                    *k = 0x58; //Numpad enter
                    k_it.next()
                } else {
                    Some(k)
                }
            });
        return keycodes;
    }
}
