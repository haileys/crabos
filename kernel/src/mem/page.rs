use bitflags::bitflags;

use crate::critical::{self, Critical};
use crate::mem::phys::{self, Phys, MemoryExhausted};

#[repr(transparent)]
struct Entry(u64);

pub const PAGE_SIZE: usize = 0x1000;

bitflags! {
    pub struct PageFlags: u64 {
        const PRESENT   = 0x001;
        const WRITE     = 0x002;
        const USER      = 0x004;
    }
}

const PAGE_DIRECTORY: *mut Entry = 0xfffff000 as *mut Entry;
const PAGE_TABLES: *mut Entry = 0xffc00000 as *mut Entry;

extern "C" {
    static mut temp_page: u8;
}

pub fn invlpg(virt: *const u8) {
    unsafe { asm!("invlpg ($0)" :: "r"(virt) : "memory" : "volatile"); }
}

// the existence of a reference to CriticalLock proves we're in a critical
// section:
pub unsafe fn temp_map<T>(phys: Phys, _critical: &Critical) -> Result<*mut T, MapError> {
    let virt = &mut temp_page as *mut u8;
    map(phys, virt, PageFlags::PRESENT | PageFlags::WRITE)?;
    Ok(virt as *mut T)
}

pub unsafe fn temp_unmap(_critical: &Critical) {
    let virt = &mut temp_page as *mut u8;
    unmap(virt).expect("unmap");
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped,
    CannotAllocatePageTable,
}

pub unsafe fn map(phys: Phys, virt: *const u8, flags: PageFlags) -> Result<(), MapError> {
    critical::section(|| {
        let virt = virt as u64;

        let pde = PAGE_DIRECTORY.add((virt >> 22) as usize);
        let pte = PAGE_TABLES.add((virt >> 12) as usize);

        if (*pde).0 == 0 {
            // need to allocate new page table for entry:
            let pt = phys::alloc().map_err(|_: MemoryExhausted|
                MapError::CannotAllocatePageTable)?;

            *pde = Entry(pt.0 | (PageFlags::PRESENT | PageFlags::WRITE).bits());
            invlpg(pte as *const u8);
        }

        if (*pte).0 != 0 {
            return Err(MapError::AlreadyMapped);
        }

        *pte = Entry(phys.0 | flags.bits());
        invlpg(virt as *const u8);

        Ok(())
    })
}

#[derive(Debug)]
pub enum UnmapError {
    NotMapped,
}

pub unsafe fn unmap(virt: *const u8) -> Result<(), UnmapError> {
    critical::section(|| {
        let virt = virt as u64;

        let pde = PAGE_DIRECTORY.add((virt >> 22) as usize);
        let pte = PAGE_TABLES.add((virt >> 12) as usize);

        if (*pde).0 == 0 {
            return Err(UnmapError::NotMapped);
        }

        if (*pte).0 == 0 {
            return Err(UnmapError::NotMapped);
        }

        *pte = Entry(0);

        Ok(())
    })
}
