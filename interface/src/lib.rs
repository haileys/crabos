#![no_std]

#[macro_use]
mod macros {
    macro_rules! enum64 {
        ( enum $enum_name:ident { $($vector:expr => $variant_name:ident,)* } ) => {
            #[derive(Debug)]
            #[repr(u64)]
            pub enum $enum_name {
                $($variant_name = $vector,)*
            }

            impl core::convert::TryFrom<u64> for $enum_name {
                type Error = ();

                fn try_from(vector: u64) -> Result<Self, Self::Error> {
                    match vector {
                        $($vector => Ok($enum_name::$variant_name),)*
                        _ => Err(()),
                    }
                }
            }
        }
    }
}

mod syscall;

pub use syscall::*;
