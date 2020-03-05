#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]

extern crate crabapi;

#[no_mangle]
pub extern "C" fn main() {
    let mut buf = [0u8; 32];
    unsafe { crabapi::syscall::read_file(1, buf.as_mut_ptr(), buf.len() as u64); }

    let buf = b"\n\nWelcome.\n\n";
    unsafe { crabapi::syscall::write_file(1, buf.as_ptr(), buf.len() as u64); }

    loop {}
}
