use bitflags::bitflags;

use crate::critical::{self, Critical};
use crate::mem::phys::{self, Phys, RawPhys, MemoryExhausted};

pub const PAGE_SIZE: usize = 0x1000;

#[repr(transparent)]
struct PmlEntry(u64);

impl PmlEntry {
    fn raw_phys(&self) -> Option<RawPhys> {
        let raw = self.0 & !0xfff;

        if raw != 0 {
            Some(RawPhys(raw))
        } else {
            None
        }
    }

    pub fn flags(&self) -> PageFlags {
        PageFlags::from_bits(self.0 & 0xfff).expect("PageFlags::from_bits in PmlEntry::flags")
    }
}

bitflags! {
    pub struct PageFlags: u64 {
        const PRESENT   = 0x001;
        const WRITE     = 0x002;
        const USER      = 0x004;
    }
}

unsafe fn pml4_entry(virt: u64) -> *mut PmlEntry {
    let base = 0xfffffffffffff000 as *mut PmlEntry;
    base.add(((virt >> 39) & 0x1ff) as usize)
}

unsafe fn pml3_entry(virt: u64) -> *mut PmlEntry {
    let base = 0xffffffffffe00000 as *mut PmlEntry;
    base.add(((virt >> 30) & 0x3ffff) as usize)
}

unsafe fn pml2_entry(virt: u64) -> *mut PmlEntry {
    let base = 0xffffffffc0000000 as *mut PmlEntry;
    base.add(((virt >> 21) & 0x7ffffff) as usize)
}

unsafe fn pml1_entry(virt: u64) -> *mut PmlEntry {
    let base = 0xffffff8000000000 as *mut PmlEntry;
    base.add(((virt >> 12) & 0xfffffffff) as usize)
}

extern "C" {
    static mut temp_page: u8;
}

pub fn invlpg(virt: *const u8) {
    unsafe { asm!("invlpg ($0)" :: "r"(virt) : "memory" : "volatile"); }
}

// the existence of a reference to CriticalLock proves we're in a critical
// section:
pub unsafe fn temp_map<T>(phys: RawPhys, _critical: &Critical) -> Result<*mut T, MapError> {
    let virt = &mut temp_page as *mut u8;
    let entry = pml1_entry(virt as u64);

    if (*entry).0 != 0 {
        panic!("temp page already mapped");
    }

    *entry = PmlEntry(phys.0 | (PageFlags::PRESENT | PageFlags::WRITE).bits());

    Ok(virt as *mut T)
}

pub unsafe fn temp_unmap(_critical: &Critical) {
    let virt = &mut temp_page as *mut u8;
    *pml1_entry(virt as u64) = PmlEntry(0);
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped,
    CannotAllocatePageTable,
}

pub unsafe fn map(phys: Phys, virt: *const u8, flags: PageFlags) -> Result<(), MapError> {
    critical::section(|| {
        let virt = virt as u64;

        let pml4_ent = pml4_entry(virt);
        let pml3_ent = pml3_entry(virt);
        let pml2_ent = pml2_entry(virt);
        let pml1_ent = pml1_entry(virt);

        // ensure all pml tables exist:

        if (*pml4_ent).0 == 0 {
            // need to allocate new page table for entry:
            let pml3_tab = phys::alloc().map_err(|_: MemoryExhausted|
                MapError::CannotAllocatePageTable)?;

            *pml4_ent = PmlEntry(pml3_tab.into_raw().0 | (PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER).bits());
            invlpg(pml3_ent as *const u8);
        }

        if (*pml3_ent).0 == 0 {
            // need to allocate new page table for entry:
            let pml2_tab = phys::alloc().map_err(|_: MemoryExhausted|
                MapError::CannotAllocatePageTable)?;

            *pml3_ent = PmlEntry(pml2_tab.into_raw().0 | (PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER).bits());
            invlpg(pml2_ent as *const u8);
        }

        if (*pml2_ent).0 == 0 {
            // need to allocate new page table for entry:
            let pml1_tab = phys::alloc().map_err(|_: MemoryExhausted|
                MapError::CannotAllocatePageTable)?;

            *pml2_ent = PmlEntry(pml1_tab.into_raw().0 | (PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER).bits());
            invlpg(pml1_ent as *const u8);
        }

        if (*pml1_ent).0 != 0 {
            return Err(MapError::AlreadyMapped);
        }

        *pml1_ent = PmlEntry(phys.into_raw().0 | flags.bits());
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

        let pml4_ent = pml4_entry(virt);
        let pml3_ent = pml3_entry(virt);
        let pml2_ent = pml2_entry(virt);
        let pml1_ent = pml1_entry(virt);

        if (*pml4_ent).0 == 0 {
            return Err(UnmapError::NotMapped);
        }

        if (*pml3_ent).0 == 0 {
            return Err(UnmapError::NotMapped);
        }

        if (*pml2_ent).0 == 0 {
            return Err(UnmapError::NotMapped);
        }

        match (*pml1_ent).raw_phys() {
            Some(raw_phys) => {
                // ensure we decrement the ref count of the physical page:
                Phys::from_raw(raw_phys);
                *pml1_ent = PmlEntry(0);
                Ok(())
            }
            None => {
                Err(UnmapError::NotMapped)
            }
        }
    })
}
