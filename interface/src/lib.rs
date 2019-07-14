#![no_std]

#[macro_use]
mod macros {
    macro_rules! enum64 {
        ( enum $enum_name:ident { $($vector:expr => $variant_name:ident,)* } ) => {
            #[derive(Debug)]
            pub enum $enum_name {
                $($variant_name,)*
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

            impl Into<u64> for $enum_name {
                fn into(self) -> u64 {
                    match self {
                        $($enum_name::$variant_name => $vector,)*
                    }
                }
            }
        }
    }
}

mod syscall;

pub use syscall::*;
