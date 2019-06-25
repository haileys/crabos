#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]
#![feature(panic_info_message)]

mod console;
mod critical;
mod device;
mod interrupt;
mod mem;
mod panic;

#[no_mangle]
pub extern "C" fn main() -> ! {
    unsafe {
        let critical = critical::begin();
        mem::page::temp_unmap(&critical);
        device::pit::init();
    }

    println!("Hello world!");

    println!("Allocating phys: {:?}", mem::phys::alloc());

    loop {
        unsafe { asm!("hlt"); }
    }
}
