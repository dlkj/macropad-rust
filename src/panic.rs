use crate::OLED_DISPLAY;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::sync::atomic::{self, Ordering};
use log::error;

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);

    let mut output = arrayvec::ArrayString::<1024>::new();
    let _r = write!(&mut output, "{}", info).map(|_| {
        cortex_m::interrupt::free(|cs| {
            let mut display_ref = OLED_DISPLAY.borrow(cs).borrow_mut();
            if let Some(display) = display_ref.as_mut() {
                let _r = display.draw_text_screen(output.as_str());
            }
        });
    });

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
