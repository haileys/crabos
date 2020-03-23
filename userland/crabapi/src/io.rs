use core::mem::ManuallyDrop;

use crate::Handle;
use crate::syscall;

pub type Error = interface::SysError;
pub type Result<T> = core::result::Result<T, Error>;

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn flush(&mut self) -> Result<()>;

    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while buf.len() > 0 {
            let written = self.write(buf)?;
            buf = &buf[written..];
        }

        Ok(())
    }
}

pub fn console() -> Console {
    const CONSOLE_HANDLE: u64 = 1;
    let handle = unsafe { Handle::from_raw(CONSOLE_HANDLE) };
    Console(ManuallyDrop::new(handle))
}

#[derive(Clone)]
pub struct Console(ManuallyDrop<Handle>);

impl Read for Console {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let result = unsafe {
            syscall::read_stream(self.0.as_raw(), buf.as_mut_ptr(), buf.len() as u64)
        };

        result.into()
    }
}

impl Write for Console {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let result = unsafe {
            syscall::write_stream(self.0.as_raw(), buf.as_ptr(), buf.len() as u64)
        };

        result.into()
    }

    fn flush(&mut self) -> Result<()> {
        // no-op on console
        Ok(())
    }
}
