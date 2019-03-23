#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]
#![feature(panic_info_message)]

mod console;
mod panic;

#[no_mangle]
pub extern "C" fn main() -> ! {
    println!("Hello world!");

    panic!("whoops");
}
