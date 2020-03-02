mod early_init;
pub use early_init::EarlyInit;

use core::iter;
use core::str::{self, Utf8Error};

use arrayvec::{Array, ArrayString};

#[derive(Debug)]
pub enum ArrayStringError {
    TooLong,
    Utf8(Utf8Error),
}

pub fn array_string<A: Copy + Array<Item = u8>>(buf: &[u8])
    -> Result<ArrayString<A>, ArrayStringError>
{
    let mut s = ArrayString::new();

    if buf.len() > s.capacity() {
        return Err(ArrayStringError::TooLong);
    }

    s.push_str(
        str::from_utf8(buf)
            .map_err(ArrayStringError::Utf8)?);

    Ok(s)
}

pub fn utf8_valid_parts(buf: &[u8]) -> impl Iterator<Item = &str> {
    let mut idx = 0;

    iter::from_fn(move || {
        match str::from_utf8(&buf[idx..]) {
            Ok("") => None,
            Ok(s) => {
                idx += s.len();
                Some(s)
            }
            Err(e) => {
                let part = str::from_utf8(&buf[idx..][..e.valid_up_to()])
                    .expect("proven str::from_utf8");
                idx += e.valid_up_to() + e.error_len().unwrap_or(0);
                Some(part)
            }
        }
    })
}
