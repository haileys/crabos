use core::fmt::Write;

use interface::{SysResult, SysError};
use itertools::Itertools;

use crate::console;
use crate::util;

#[derive(Debug)]
pub enum File {
    Console,
}

impl File {
    pub async fn write(&self, buf: &[u8]) -> SysResult<usize> {
        match self {
            File::Console => {
                let mut con = console::get();

                util::utf8_valid_parts(buf)
                    .intersperse("?")
                    .map(|part| con.write_str(part)
                        .map_err(|_| SysError::IoError))
                    .collect::<Result<(), SysError>>()?;

                Ok(buf.len())
            }
        }
    }
}
