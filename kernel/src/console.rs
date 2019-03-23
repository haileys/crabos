use core::fmt::{self, Write};

// TODO - unsafe. we need to lock around access to PortE9
pub struct PortE9;

impl Write for PortE9 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.as_bytes() {
            unsafe { asm!("outb %al, %dx" :: "{dx}"(0xe9), "{al}"(*b) :: "volatile"); }
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::write_fmt(format_args!($($arg)*)));
}

pub fn write_fmt(args: fmt::Arguments) {
    let _ = fmt::write(&mut PortE9, args);
}
