use arrayvec::ArrayString;
use core::fmt::{self, Write};
use core::panic::PanicInfo;

use crate::syscall;

const CONSOLE_HANDLE: u64 = 1;

unsafe fn write_bytes(buf: &[u8]) {
    syscall::write_file(CONSOLE_HANDLE, buf.as_ptr(), buf.len() as u64);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        match info.message() {
            Some(args) => {
                write_bytes(b"panic: ");
                let mut buff = ArrayString::<[u8; 1024]>::new();
                let _ = fmt::write(&mut buff, *args);
                write_bytes(buff.as_bytes());
                write_bytes(b"\n");
            }
            None => {
                write_bytes(b"panic\n");
            }
        }

        if let Some(loc) = info.location() {
            write_bytes(b" at ");
            write_bytes(loc.file().as_bytes());
            write_bytes(b":");

            let mut buff = ArrayString::<[u8; 16]>::new();
            let _ = write!(&mut buff, "{}", loc.line());
            write_bytes(buff.as_bytes());

            write_bytes(b"\n");
        }

        syscall::exit(u64::max_value());
        loop {}
    }
}
