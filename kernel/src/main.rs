#![no_std]
#![no_main]
#![feature(alloc)]
#![feature(alloc_error_handler)]
#![feature(asm)]
#![feature(core_panic)]
#![feature(panic_info_message)]

extern crate alloc;

mod console;
mod critical;
mod device;
mod interrupt;
mod mem;
mod panic;
mod sync;

use mem::kvirt::WatermarkAllocator;

extern "C" {
    static mut end: u8;
}

#[global_allocator]
pub static DEFAULT_ALLOCATOR: WatermarkAllocator = unsafe {
    WatermarkAllocator::new(&end as *const u8 as *mut u8)
};

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
