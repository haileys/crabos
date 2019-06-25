#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(asm)]
#![feature(core_panic)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(ptr_offset_from)]

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
    static mut _end: u8;
}

#[global_allocator]
pub static DEFAULT_ALLOCATOR: WatermarkAllocator = unsafe {
    WatermarkAllocator::new(&_end as *const u8 as *mut u8)
};

pub fn bar() {
    panic::trace();
}

pub fn foo() {
    bar();
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    unsafe {
        let critical = critical::begin();

        // init temp mapping
        mem::page::temp_unmap(&critical);

        // init pit
        device::pit::init();
    }

    println!("Hello world!");

    foo();

    loop {
        // unsafe { asm!("hlt"); }
    }
}
