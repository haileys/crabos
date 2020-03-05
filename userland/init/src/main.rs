#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]
// #![feature(start)]

extern crate crabapi;

// #[start]
#[export_name = "main_"]
pub extern "C" fn main_(/*_: isize, _: *const *const u8*/) -> isize {
    panic!("Hello world");
    unsafe { asm!("xchgw %cx, %cx" :::: "volatile"); }
    loop {}
}
