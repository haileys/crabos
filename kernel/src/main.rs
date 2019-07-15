#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(ptr_offset_from)]


mod console;
mod critical;
mod device;
mod interrupt;
mod mem;
mod panic;
mod sync;
mod syscall;
mod task;

use core::ptr;

use mem::phys;
use mem::page::{self, PageFlags};

extern "C" {
    static mut _end: u8;
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    unsafe {
        let crit = critical::begin();

        // perform follow up init for phys allocator
        phys::init_ref_counts(&crit);

        // init pit
        device::pit::init();
    }

    let a_bin = include_bytes!("../../target/x86_64-kernel/userland/a.bin");
    let a_addr = 0x1_0000_0000 as *mut u8;

    let b_bin = include_bytes!("../../target/x86_64-kernel/userland/b.bin");
    let b_addr = 0x1_0000_1000 as *mut u8;

    unsafe {
        let phys = phys::alloc()
            .expect("phys::alloc");

        page::map(phys, a_addr, PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER)
            .expect("page::map");

        ptr::copy(a_bin.as_ptr(), a_addr, a_bin.len());

        let phys = phys::alloc()
            .expect("phys::alloc");

        page::map(phys, b_addr, PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER)
            .expect("page::map");

        ptr::copy(b_bin.as_ptr(), b_addr, b_bin.len());

        task::start();
    }
}
