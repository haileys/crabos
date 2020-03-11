use core::fmt::Write;

use interface::{SysError, SysResult};
use itertools::Itertools;

use crate::fs::fat16::{self, Fat16, DirEntry, FatError};
use crate::util;

pub use fat16::Open;

#[derive(Debug)]
pub struct Filesystem {
    root: Fat16,
}

#[derive(Debug)]
pub enum OpenError {
    NotFound,
    Fat(FatError),
}

impl From<OpenError> for SysError {
    fn from(e: OpenError) -> Self {
        match e {
            OpenError::NotFound => SysError::NoFile,
            OpenError::Fat(e) => e.into(),
        }
    }
}

impl Filesystem {
    pub fn new(root: Fat16) -> Self {
        Filesystem { root }
    }

    pub async fn open(&self, path: &[u8]) -> Result<File, OpenError> {
        let mut container = self.root.root();
        let mut segments = path.split(|b| *b == b'/');

        // ensure path starts with /:
        if segments.next() != Some(b"") {
            // TODO support relative paths
            return Err(OpenError::NotFound);
        }

        loop {
            let segment = match segments.next() {
                None => {
                    return Ok(File::Fat(Open::Dir(container)));
                }
                Some(b"") => {
                    // ignore empty path segments
                    continue;
                }
                Some(segment) => segment,
            };

            let entry = container.entry(segment)
                .await
                .map_err(OpenError::Fat)?
                .ok_or(OpenError::NotFound)?;

            match entry.open() {
                Ok(Open::Dir(dir)) => {
                    container = dir;
                }
                Ok(Open::File(file)) => {
                    match segments.next() {
                        None => {
                            // this was the last path segment, return file
                            return Ok(File::Fat(Open::File(file)));
                        }
                        Some(_) => {
                            // there are more path segments to go, and a file
                            // cannot possibly contain directory entries
                            return Err(OpenError::NotFound);
                        }
                    }
                }
                Err(e) => {
                    return Err(OpenError::Fat(e));
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum File {
    Console,
    Fat(Open),
}

impl File {
    pub async fn read(&self, buf: &mut [u8]) -> SysResult<usize> {
        match self {
            File::Console => {
                use crate::device::keyboard;

                if buf.len() == 0 {
                    return Ok(0);
                }

                crate::println!("before read_scancode");

                let scancode = keyboard::read_scancode().await;
                buf[0] = scancode;
                Ok(1)
            }
            File::Fat(Open::File(file)) => {
                Ok(file.read(buf).await?)
            }
            File::Fat(Open::Dir(_)) => {
                Err(SysError::InvalidOperation)
            }
        }
    }

    pub async fn write(&self, buf: &[u8]) -> SysResult<usize> {
        match self {
            File::Console => {
                use crate::console;

                let mut con = console::get();

                util::utf8_valid_parts(buf)
                    .intersperse("?")
                    .map(|part| con.write_str(part)
                        .map_err(|_| SysError::IoError))
                    .collect::<Result<(), SysError>>()?;

                Ok(buf.len())
            }
            File::Fat(_) => { panic!() }
        }
    }
}
