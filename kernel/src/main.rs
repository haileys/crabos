#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]
#![feature(panic_info_message)]

mod panic;

#[no_mangle]
pub extern "C" fn main() -> ! {
    panic!("whoops");
}
