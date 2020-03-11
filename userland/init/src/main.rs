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

    let path = b"/init.bin";
    let handle = unsafe { crabapi::syscall::open_file(path.as_ptr(), path.len() as u64, 0) };
    unsafe { asm!("xchgw %bx, %bx" :: "{rax}"(handle)); }

    let mut buf = [0u8; 128];
    let res = unsafe { crabapi::syscall::read_file(handle, buf.as_mut_ptr(), buf.len() as u64) };
    unsafe { asm!("xchgw %bx, %bx" :: "{rax}"(res)); }

    let res = unsafe { crabapi::syscall::write_file(1, buf.as_ptr(), 128) };
    unsafe { asm!("xchgw %bx, %bx" :: "{rax}"(res)); }

    loop {}
}
