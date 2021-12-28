#![no_std]
#![no_main]

//USB serial console (minicom -b 115200 -o -D /dev/ttyACM0)

mod keyboard;
mod logger;
mod neopixel;
mod oled_display;
mod panic;
mod rotary_enc;
mod usb;

use adafruit_macropad::{
    hal::{
        self as rp2040_hal,
        clocks::Clock,
        pac::{self, interrupt},
        pio::PIOExt,
        sio::Sio,
        timer::Timer,
        watchdog::Watchdog,
    },
    Pins,
};
use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::prelude::*;
use embedded_time::duration::Extensions;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::rate::Hertz;
use keyboard::keycode::KeyCode;
use keyboard::Keyboard;
use log::{info, LevelFilter};
use rp2040_hal::gpio::dynpin::DynPin;
use sh1106::{prelude::*, Builder};
use usb_device::class_prelude::*;
use usbd_hid::descriptor::KeyboardReport;
use ws2812_pio::Ws2812;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;

type Spi = rp2040_hal::spi::Spi<rp2040_hal::spi::Enabled, rp2040_hal::pac::SPI1, 8_u8>;
type OledDisplay = oled_display::OledDisplay<sh1106::interface::SpiInterface<Spi, DynPin, DynPin>>;

static USB_MANAGER: Mutex<RefCell<Option<usb::UsbManager<rp2040_hal::usb::UsbBus>>>> =
    Mutex::new(RefCell::new(None));
static LOGGER: logger::MacropadLogger = logger::MacropadLogger;
static OLED_DISPLAY: Mutex<RefCell<Option<OledDisplay>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks: rp2040_hal::clocks::ClocksManager = rp2040_hal::clocks::init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    //init neopixels
    let mut neopixel: neopixel::Neopixels<_, 12> = {
        let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

        let ws = Ws2812::new(
            pins.neopixel.into_mode(),
            &mut pio,
            sm0,
            clocks.peripheral_clock.freq(),
            timer.count_down(),
        );
        neopixel::Neopixels::new(ws)
    };

    //init the oled display
    {
        // These are implicitly used by the spi driver if they are in the correct mode
        let _spi_sclk = pins.sclk.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let _spi_mosi = pins.mosi.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let _spi_miso = pins.miso.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let spi = rp2040_hal::spi::Spi::<_, _, 8>::new(pac.SPI1);

        // Display control pins
        let oled_dc = pins.oled_dc.into_push_pull_output();
        let oled_cs = pins.oled_cs.into_push_pull_output();
        let mut oled_reset = pins.oled_reset.into_push_pull_output();

        let res = oled_reset.set_high();
        res.unwrap(); //disable screen reset

        // Exchange the uninitialised SPI driver for an initialised one
        let oled_spi = spi.init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            Hertz::new(16_000_000u32),
            &embedded_hal::spi::MODE_0,
        );

        let mut display: GraphicsMode<_> = Builder::new()
            .connect_spi(oled_spi, oled_dc.into(), oled_cs.into())
            .into();

        display.init().unwrap();
        display.flush().unwrap();

        cortex_m::interrupt::free(|cs| {
            OLED_DISPLAY
                .borrow(cs)
                .replace(Some(OledDisplay::new(display)));
        });
    }

    cortex_m::interrupt::free(|cs| {
        // Note (safety): interupts not yet enabled

        //Init USB
        static mut USB_BUS: Option<UsbBusAllocator<rp2040_hal::usb::UsbBus>> = None;

        unsafe {
            USB_BUS = Some(UsbBusAllocator::new(rp2040_hal::usb::UsbBus::new(
                pac.USBCTRL_REGS,
                pac.USBCTRL_DPRAM,
                clocks.usb_clock,
                true,
                &mut pac.RESETS,
            )));

            USB_MANAGER
                .borrow(cs)
                .replace(Some(usb::UsbManager::new(USB_BUS.as_ref().unwrap())));

            log::set_logger_racy(&LOGGER).unwrap();
        }
    });

    log::set_max_level(LevelFilter::Info);

    // Enable the USB interrupt
    unsafe {
        pac::NVIC::unmask(rp2040_hal::pac::Interrupt::USBCTRL_IRQ);
    };

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());

    //Rust logo splash screen
    cortex_m::interrupt::free(|cs| {
        let mut oled_display_ref = OLED_DISPLAY.borrow(cs).borrow_mut();
        let oled_display = oled_display_ref.as_mut().unwrap();
        oled_display
            .draw_image(include_bytes!("./rust.raw"), 64)
            .unwrap();
    });

    delay.delay_ms(250);

    info!("macropad starting");

    let rot_pin_a =
        debounce::DebouncedPin::<DynPin>::new(pins.encoder_rota.into_pull_up_input().into(), true);
    let rot_pin_b =
        debounce::DebouncedPin::<DynPin>::new(pins.encoder_rotb.into_pull_up_input().into(), true);

    let mut rot_enc = rotary_enc::RotaryEncoder::new(rot_pin_a, rot_pin_b);

    let pins: [DynPin; 12] = [
        pins.key1.into_pull_up_input().into(),
        pins.key2.into_pull_up_input().into(),
        pins.key3.into_pull_up_input().into(),
        pins.key4.into_pull_up_input().into(),
        pins.key5.into_pull_up_input().into(),
        pins.key6.into_pull_up_input().into(),
        pins.key7.into_pull_up_input().into(),
        pins.key8.into_pull_up_input().into(),
        pins.key9.into_pull_up_input().into(),
        pins.key10.into_pull_up_input().into(),
        pins.key11.into_pull_up_input().into(),
        pins.key12.into_pull_up_input().into(),
    ];

    //keypad, final row: '0', '.', 'enter'
    const KEY_MAP: [keyboard::KeyAction; 12] = [
        keyboard::KeyAction::Key { code: KeyCode::Kp7 },
        keyboard::KeyAction::Key { code: KeyCode::Kp8 },
        keyboard::KeyAction::Key { code: KeyCode::Kp9 },
        keyboard::KeyAction::Key { code: KeyCode::Kp4 },
        keyboard::KeyAction::Key { code: KeyCode::Kp5 },
        keyboard::KeyAction::Key { code: KeyCode::Kp6 },
        keyboard::KeyAction::Key { code: KeyCode::Kp1 },
        keyboard::KeyAction::Key { code: KeyCode::Kp2 },
        keyboard::KeyAction::Key { code: KeyCode::Kp3 },
        keyboard::KeyAction::Key { code: KeyCode::Kp0 },
        keyboard::KeyAction::Key {
            code: KeyCode::KpDot,
        },
        keyboard::KeyAction::Key {
            code: KeyCode::KpEnter,
        },
    ];

    let mut keyboard = Keyboard::new(
        keyboard::DirectPinMatrix::new(pins),
        keyboard::BasicKeyboardLayout::new(KEY_MAP),
    );

    let mut fast_countdown = timer.count_down();
    fast_countdown.start(1.milliseconds());

    let mut slow_countdown = timer.count_down();
    slow_countdown.start(20.milliseconds());

    info!("Running main loop");

    loop {
        //1ms scan the keys and debounce
        if fast_countdown.wait().is_ok() {
            let (p_a, p_b) = rot_enc.pins_borrow_mut();
            p_a.update().expect("Failed to update rot a debouncer");
            p_b.update().expect("Failed to update rot b debouncer");
            //todo: move onto an interupt timer
            rot_enc.update();

            keyboard.update().expect("Failed to update keyboard");
        }

        //10ms
        if slow_countdown.wait().is_ok() {
            //100Hz or slower
            let keyboard_state = keyboard.state().expect("Failed to get Keyboard state");
            let keyboard_report = get_hid_report(&keyboard_state);

            //todo - spin lock until usb ready to recive, reset timers
            cortex_m::interrupt::free(|cs| {
                let mut usb_ref = USB_MANAGER.borrow(cs).borrow_mut();
                if let Some(usb) = usb_ref.as_mut() {
                    usb.keyboard_borrow_mut().push_input(&keyboard_report).ok();
                }
            });

            //update the screen
            cortex_m::interrupt::free(|cs| {
                let mut oled_display_ref = OLED_DISPLAY.borrow(cs).borrow_mut();
                if let Some(oled_display) = oled_display_ref.as_mut() {
                    oled_display.draw_numpad(rot_enc.value()).unwrap();
                }
            });

            //update the LEDs
            let pressed_keys = keyboard_state
                .keys
                .iter()
                .map(|k| k.pressed)
                .collect::<arrayvec::ArrayVec<bool, 12>>();

            neopixel
                .update(&pressed_keys, (rot_enc.value() * 10) + 128)
                .unwrap();
        }
    }
}

fn get_hid_report<const N: usize>(state: &keyboard::KeyboardState<N>) -> KeyboardReport {
    //get first 6 current keypresses and send to usb
    let mut keycodes: [u8; 6] = [0, 0, 0, 0, 0, 0];

    let mut keycodes_it = keycodes.iter_mut();

    for k in &state.keycodes {
        match keycodes_it.next() {
            Some(kc) => {
                *kc = *k as u8;
            }
            None => {
                keycodes.fill(0x01); //Error roll over
                break;
            }
        }
    }

    KeyboardReport {
        modifier: state.modifiers.bits(),
        leds: 0,
        reserved: 0,
        keycodes,
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| {
        let mut usb_ref = USB_MANAGER.borrow(cs).borrow_mut();
        if let Some(usb) = usb_ref.as_mut() {
            usb.service_irq();
        }
    });
}
