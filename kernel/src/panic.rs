use core::fmt::{self, Write};
use core::iter::Iterator;
use core::panic::{PanicInfo, Location};
use core::panicking;
use core::slice;
use core::str;
use core::sync::atomic::{AtomicBool, Ordering};

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

static PANICKING: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let crit = critical::begin();
    let mut con = console::failsafe(&crit);

    let recursive_panic = PANICKING.swap(true, Ordering::SeqCst);

    if recursive_panic {
        let _ = con.write_str("\n\n*** PANIC while panicking, halt\n\n");
    } else {
        panic_write(&mut con, info);
    }

    unsafe { asm!("cli; hlt") };
    loop {}
}

#[export_name = "panic"]
pub unsafe extern "C" fn c_panic(msg: *const u8) -> ! {
    // find null terminator:
    let msg_len = (0..).find(|idx| *msg.add(*idx) == 0)
        .expect("panic msg must have null terminator");
    let bytes = slice::from_raw_parts(msg, msg_len);
    let msg = str::from_utf8_unchecked(bytes);

    let loc = Location::internal_constructor("(none)", 0, 0);

    panicking::panic_fmt(format_args!("{}", msg), &loc);
}
