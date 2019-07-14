use core::fmt::{self, Write};
use core::marker::PhantomData;

use crate::critical::Critical;

pub fn get<'a>(_crit: &'a Critical) -> impl Write + 'a {
    PortE9 { crit: PhantomData }
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let crit = $crate::critical::begin();
        let _ = $crate::console::write_fmt(format_args!($($arg)*), &crit);
    }};
}

pub fn write_fmt(args: fmt::Arguments, crit: &Critical) {
    let mut con = get(&crit);
    let _ = fmt::write(&mut con, args);
}

struct PortE9<'a> {
    crit: PhantomData<&'a Critical>,
}

impl<'a> Write for PortE9<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.as_bytes() {
            unsafe { asm!("outb %al, %dx" :: "{dx}"(0xe9), "{al}"(*b) :: "volatile"); }
        }

        Ok(())
    }
}
