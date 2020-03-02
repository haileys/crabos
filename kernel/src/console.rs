use core::fmt::{self, Write};

use crate::critical::Critical;
use crate::sync::{Mutex, MutexGuard};

mod vga;

static CONSOLE: Mutex<Console> = Mutex::new(Console::PortE9(PortE9));

pub(self) enum Console {
    PortE9(PortE9),
    VgaText(vga::VgaText),
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            Console::PortE9(con) => con.write_str(s),
            Console::VgaText(con) => con.write_str(s),
        }
    }
}

pub fn get() -> MutexGuard<'static, impl Write> {
    CONSOLE.lock()
}

pub(self) fn set(console: Console) {
    *CONSOLE.lock() = console;
}

pub fn failsafe<'a>(_crit: &'a Critical) -> impl Write + 'a {
    PortE9
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let _ = $crate::console::write_fmt(format_args!($($arg)*));
    }};
}

pub fn write_fmt(args: fmt::Arguments) {
    let mut con = get();
    let _ = fmt::write(&mut *con, args);
}

struct PortE9;

impl Write for PortE9 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            unsafe { asm!("outb %al, %dx" :: "{dx}"(0xe9), "{al}"(b) :: "volatile"); }
        }

        Ok(())
    }
}
