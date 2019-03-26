use core::fmt::{self, Write};
use core::panic::PanicInfo;

use crate::console;
use crate::critical;

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
    let crit = critical::begin();
    let mut con = console::get(&crit);
    panic_write(&mut con, info);

    unsafe { asm!("cli; hlt") };
    loop {}
}
