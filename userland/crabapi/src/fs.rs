use crate::Handle;
use crate::io::{Result, Read, Write};
use crate::syscall;

#[derive(Clone)]
pub struct File(Handle);

impl File {
    pub fn open(path: &[u8]) -> Result<File> {
        let ret = unsafe {
            syscall::open_file(path.as_ptr(), path.len() as u64, 0)
        };

        Result::from(ret).map(File)
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let result = unsafe {
            syscall::read_file(self.0.as_raw(), buf.as_mut_ptr(), buf.len() as u64)
        };

        result.into()
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let result = unsafe {
            syscall::write_file(self.0.as_raw(), buf.as_ptr(), buf.len() as u64)
        };

        result.into()
    }

    fn flush(&mut self) -> Result<()> {
        // no-op on console
        Ok(())
    }
}
