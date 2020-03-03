#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(ptr_offset_from)]
#![feature(allocator_api)]
#![feature(never_type)]
#![feature(ptr_internals)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(panic_internals)]
#![feature(arbitrary_self_types)]

#[allow(unused)]
#[macro_use]
extern crate kernel_derive;

mod console;
mod critical;
mod device;
mod fs;
mod interrupt;
mod mem;
mod object;
mod panic;
mod sync;
mod syscall;
mod task;
mod util;

use core::slice;

use interrupt::TrapFrame;
use mem::page::{self, PageFlags, PAGE_SIZE};
use mem::phys;
use object::ObjectRef;

extern "C" {
    static mut _end: u8;
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    unsafe {
        let crit = critical::begin();

        // perform follow up init for phys allocator
        phys::init_ref_counts(&crit);

        // init kernel PML4 entries
        mem::page::init_kernel_pml4_entries(&crit);

        // init object space
        object::init();

        // init pit
        device::pit::init();

        // init keyboard
        device::keyboard::init();
    }

    task::init();

    unsafe {
        let page_ctx = ObjectRef::new(page::current_ctx())
            .expect("ObjectRef::new");

        task::spawn(page_ctx, |task| async move {
            use device::ide::{self, Drive};
            use device::mbr::Mbr;
            use fs::fat16::{Fat16, DirectoryEntry};

            let ide = ide::PRIMARY.open(Drive::A)
                .expect("ide::open");

            println!("detecting primary master...");
            println!("---> {:?}", ide.detect().await);

            let mbr = Mbr::open(ide)
                .expect("Mbr::open");

            let mut partitions = mbr.partitions().await
                .expect("mbr.partitions");

            for part in partitions.iter() {
                if let Some(part) = part {
                    crate::println!("#{} - {}, {}", part.number, part.lba, part.sectors);
                }
            }

            let fat = Fat16::open(partitions.remove(0).expect("partitions[0]")).await
                .expect("Fat16::open");

            // find init:
            let entry = fat.root().entry(b"init.bin")
                .await
                .expect("Directory::entry");

            let init = match entry {
                Some(DirectoryEntry::File(init)) => init,
                Some(DirectoryEntry::Dir(_)) => {
                    panic!("/init.bin is directory");
                }
                None => {
                    panic!("/init.bin does not exist");
                }
            };

            // setup init task
            let mut addr = 0x1_0000_0000 as *mut u8;
            let mut task = task.setup(TrapFrame::new(addr as u64, 0x0));

            let mut init = init.open();

            // read init into userspace
            loop {
                let phys = phys::alloc()
                    .expect("phys::alloc");

                page::map(phys, addr, PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER)
                    .expect("page::map");

                let read = init.read(slice::from_raw_parts_mut(addr, PAGE_SIZE))
                    .await
                    .expect("init.read");

                addr = addr.add(read);

                if read < PAGE_SIZE {
                    break;
                }
            }

            // set up initial console object
            let console = ObjectRef::new(object::file::File::Console)
                .expect("ObjectRef::new");

            object::put(task::current(), console.as_dyn()) // implicitly handle 1
                .expect("object::put");

            task.run_loop().await;
        }).expect("task::spawn init");

        // task::spawn(|task| async move {
        //     let mut task = task.setup(TrapFrame::new(b_addr as u64, 0x0));

        //     let phys = phys::alloc()
        //         .expect("phys::alloc");

        //     page::map(phys, b_addr, PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER)
        //         .expect("page::map");

        //     ptr::copy(b_bin.as_ptr(), b_addr, b_bin.len());

        //     task.run_loop().await;
        // }).expect("task::spawn second");

        task::start();
    }
}

#[no_mangle]
pub extern "C" fn __tls_get_addr() {
    panic!("__tls_get_addr not implemented");
}
