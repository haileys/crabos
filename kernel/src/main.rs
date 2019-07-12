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

use core::ptr;

use mem::kvirt::WatermarkAllocator;
use mem::phys;
use mem::page::{self, PageFlags};

extern "C" {
    static mut _end: u8;
}

#[global_allocator]
pub static DEFAULT_ALLOCATOR: WatermarkAllocator = unsafe {
    WatermarkAllocator::new(&_end as *const u8 as *mut u8)
};

#[no_mangle]
pub extern "C" fn main() -> ! {
    unsafe {
        let crit = critical::begin();

        // perform follow up init for phys allocator
        phys::init_ref_counts(&crit);

        // init pit
        device::pit::init();
    }

    let user_bin = include_bytes!("../../target/x86_64-kernel/userland/init.bin");
    let user_addr = 0x1_0000_0000 as *mut u8;

    unsafe {
        let phys = phys::alloc()
            .expect("phys::alloc");

        page::map(phys, user_addr, PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER)
            .expect("page::map");

        ptr::copy(user_bin.as_ptr(), user_addr, user_bin.len());

        asm!(r#"
            push $$0x23
            push $$0x0
            pushf
            push $$0x1b
            push $0
            iretq
        "#
            :: "r"(user_addr)
            :: "volatile"
        );
    }

    loop {}
}
