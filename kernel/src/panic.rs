pub mod unwind;

use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::panicking;
use core::slice;
use core::str;
use core::iter::Iterator;

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

pub fn trace() {
    let crit = critical::begin();
    let mut console = console::get(&crit);
    unwind::trace(&mut console);
}

#[export_name = "panic"]
pub unsafe extern "C" fn c_panic(msg: *const u8) -> ! {
    // find null terminator:
    let msg_len = (0..).find(|idx| *msg.add(*idx) == 0)
        .expect("panic msg must have null terminator");

    let bytes = slice::from_raw_parts(msg, msg_len);

    let msg = str::from_utf8_unchecked(bytes);

    panicking::panic(&(msg, "(none)", 0, 0));
}
