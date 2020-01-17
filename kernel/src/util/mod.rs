mod early_init;

pub use early_init::EarlyInit;

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
