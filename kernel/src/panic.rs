use core::fmt::{self, Write};
use core::panic::PanicInfo;

use crate::console::PortE9;

enum PanicMethod {
    PortE9,
}

static METHOD: PanicMethod = PanicMethod::PortE9;

fn panic_write(mut writer: impl Write, info: &PanicInfo) {
    let _ = write!(&mut writer, "\n");

    match info.message() {
        Some(args) => {
            let _ = write!(&mut writer, "*** PANIC: ");
            let _ = fmt::write(&mut writer, *args);
            let _ = write!(&mut writer, "\n");
        }
        None => {
            let _ = write!(&mut writer, "*** PANIC\n");
        }
    }

    if let Some(loc) = info.location() {
        let _ = write!(&mut writer, "    at {}:{}\n", loc.file(), loc.line());
    }

    let _ = write!(&mut writer, "\n");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    match METHOD {
        PanicMethod::PortE9 => panic_write(PortE9, info)
    }

    unsafe { asm!("cli; hlt") };
    loop {}
}
