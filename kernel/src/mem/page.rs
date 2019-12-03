use core::ptr;

use bitflags::bitflags;
use x86_64::registers::control::Cr3;

use crate::critical::{self, Critical};
use crate::mem::MemoryExhausted;
use crate::mem::phys::{self, Phys, RawPhys};

pub const PAGE_SIZE: usize = 0x1000;

#[repr(transparent)]
pub struct PmlEntry(pub u64);

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

    pub fn set_flags(&mut self, flags: PageFlags) {
        let new_entry = (self.0 & !0xfff) | flags.bits();
        self.0 = new_entry;
    }
}

bitflags! {
    pub struct PageFlags: u64 {
        const PRESENT   = 0x001;
        const WRITE     = 0x002;
        const USER      = 0x004;
    }
}

#[derive(Clone)]
pub struct PageCtx {
    pml4: Phys,
}

impl PageCtx {
    pub fn new() -> Result<Self, MemoryExhausted> {
        let pml4_raw = phys::alloc()?.into_raw();

        unsafe {
            let crit = critical::begin();
            let pml4_map = &mut *temp_map::<[PmlEntry; 512]>(pml4_raw, &crit);

            ptr::copy(0xfffffffffffff800 as *const PmlEntry, pml4_map[256..511].as_mut_ptr(), 255);

            // set up recursive map entry:
            pml4_map[511] = PmlEntry(pml4_raw.0 | (PageFlags::PRESENT | PageFlags::WRITE).bits());

            temp_unmap(&crit);
        }

        let pml4 = unsafe { Phys::from_raw(pml4_raw) };
        Ok(PageCtx { pml4 })
    }
}

pub unsafe fn init_kernel_pml4_entries(crit: &Critical) {
    let kernel_start = 0xfffffffffffff800 as *mut PmlEntry;

    for i in 0..255 {
        let ent = kernel_start.offset(i);

        if (*ent).0 == 0 {
            let tab = phys::alloc().expect("phys::alloc");

            *ent = PmlEntry(tab.into_raw().0 | (PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER).bits());
        }
    }
}

pub fn current_ctx() -> PageCtx {
    let cr3;
    unsafe { asm!("movq %cr3, $0" : "=r"(cr3)); }

    let pml4 = unsafe { Phys::new(cr3) };
    PageCtx { pml4 }
}

pub unsafe fn set_ctx(ctx: PageCtx) {
    let old_cr3;
    asm!("movq %cr3, $0" : "=r"(old_cr3));

    let new_cr3 = ctx.pml4.into_raw();
    asm!("movq $0, %cr3" :: "r"(new_cr3));

    // ensure we decrement the ref count of the old previous cr3
    Phys::from_raw(old_cr3);
}

const CURRENT_PML: u64 = 0xffffff8000000000;

unsafe fn pml4_entry(base: u64, virt: u64) -> *mut PmlEntry {
    let base = (base + 0x7ffffff000) as *mut PmlEntry;
    base.add(((virt >> 39) & 0x1ff) as usize)
}

unsafe fn pml3_entry(base: u64, virt: u64) -> *mut PmlEntry {
    let base = (base + 0x7fffe00000) as *mut PmlEntry;
    base.add(((virt >> 30) & 0x3ffff) as usize)
}

unsafe fn pml2_entry(base: u64, virt: u64) -> *mut PmlEntry {
    let base = (base + 0x7fc0000000) as *mut PmlEntry;
    base.add(((virt >> 21) & 0x7ffffff) as usize)
}

unsafe fn pml1_entry(base: u64, virt: u64) -> *mut PmlEntry {
    let base = base as *mut PmlEntry;
    base.add(((virt >> 12) & 0xfffffffff) as usize)
}

/// recursively iterates PML4 yielding raw physical pages
pub unsafe fn each_phys(mut f: impl FnMut(RawPhys)) {
    f(RawPhys(Cr3::read().0.start_address().as_u64()));

    // skip recursive map entry
    for pml4_idx in 0..511 {
        let base = 0xfffffffffffff000 as *mut PmlEntry;
        let entry = &*base.add(pml4_idx);

        if let Some(phys) = entry.raw_phys() {
            f(phys);

            for pml3_idx in 0..512 {
                let base = 0xffffffffffe00000 as *mut PmlEntry;
                let entry = &*base.add((pml4_idx << 9) | pml3_idx);

                if let Some(phys) = entry.raw_phys() {
                    f(phys);

                    for pml2_idx in 0..512 {
                        let base = 0xffffffffc0000000 as *mut PmlEntry;
                        let entry = &*base.add((pml4_idx << 18) | (pml3_idx << 9) | pml2_idx);

                        if let Some(phys) = entry.raw_phys() {
                            f(phys);

                            for pml1_idx in 0..512 {
                                let base = 0xffffff8000000000 as *mut PmlEntry;
                                let entry = &*base.add((pml4_idx << 27) | (pml3_idx << 18) | (pml2_idx << 9) | pml1_idx);

                                if let Some(phys) = entry.raw_phys() {
                                    f(phys);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

extern "C" {
    static mut temp_page: u8;
}

pub fn invlpg(virt: *mut u8) {
    unsafe { asm!("invlpg ($0)" :: "r"(virt) : "memory" : "volatile"); }
}

// the existence of a reference to CriticalLock proves we're in a critical
// section:
pub unsafe fn temp_map<T>(phys: RawPhys, _critical: &Critical) -> *mut T {
    let virt = &mut temp_page as *mut u8;
    let entry = pml1_entry(CURRENT_PML, virt as u64);

    if (*entry).0 != 0 {
        panic!("temp page already mapped");
    }

    *entry = PmlEntry(phys.0 | (PageFlags::PRESENT | PageFlags::WRITE).bits());
    invlpg(virt);

    virt as *mut T
}

pub unsafe fn temp_unmap(_critical: &Critical) {
    let virt = &mut temp_page as *mut u8;
    *pml1_entry(CURRENT_PML, virt as u64) = PmlEntry(0);
    invlpg(virt);
}

#[derive(Debug)]
pub enum MapError {
    AlreadyMapped,
    CannotAllocatePageTable,
}

pub fn is_mapped(virt: *const u8) -> bool {
    critical::section(|| {
        let virt = virt as u64;

        unsafe {
            let pml4_ent = pml4_entry(CURRENT_PML, virt);
            let pml3_ent = pml3_entry(CURRENT_PML, virt);
            let pml2_ent = pml2_entry(CURRENT_PML, virt);
            let pml1_ent = pml1_entry(CURRENT_PML, virt);

            // ensure all pml tables exist:

            if (*pml4_ent).0 == 0 {
                return false;
            }

            if (*pml3_ent).0 == 0 {
                return false;
            }

            if (*pml2_ent).0 == 0 {
                return false;
            }

            (*pml1_ent).0 != 0
        }
    })
}

pub unsafe fn map(phys: Phys, virt: *mut u8, flags: PageFlags) -> Result<(), MapError> {
    crate::println!("page::map");

    critical::section(|| {
        let virt = virt as u64;

        let pml4_ent = pml4_entry(CURRENT_PML, virt);
        let pml3_ent = pml3_entry(CURRENT_PML, virt);
        let pml2_ent = pml2_entry(CURRENT_PML, virt);
        let pml1_ent = pml1_entry(CURRENT_PML, virt);

        // ensure all pml tables exist:

        if (*pml4_ent).0 == 0 {
            // need to allocate new page table for entry:
            let pml3_tab = phys::alloc().map_err(|_: MemoryExhausted|
                MapError::CannotAllocatePageTable)?;

            *pml4_ent = PmlEntry(pml3_tab.into_raw().0 | (PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER).bits());
            invlpg(pml3_ent as *mut u8);
        }

        if (*pml3_ent).0 == 0 {
            // need to allocate new page table for entry:
            let pml2_tab = phys::alloc().map_err(|_: MemoryExhausted|
                MapError::CannotAllocatePageTable)?;

            *pml3_ent = PmlEntry(pml2_tab.into_raw().0 | (PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER).bits());
            invlpg(pml2_ent as *mut u8);
        }

        if (*pml2_ent).0 == 0 {
            // need to allocate new page table for entry:
            let pml1_tab = phys::alloc().map_err(|_: MemoryExhausted|
                MapError::CannotAllocatePageTable)?;

            *pml2_ent = PmlEntry(pml1_tab.into_raw().0 | (PageFlags::PRESENT | PageFlags::WRITE | PageFlags::USER).bits());
            invlpg(pml1_ent as *mut u8);
        }

        if (*pml1_ent).0 != 0 {
            return Err(MapError::AlreadyMapped);
        }

        *pml1_ent = PmlEntry(phys.into_raw().0 | flags.bits());
        invlpg(virt as *mut u8);

        Ok(())
    })
}

#[derive(Debug)]
pub struct NotMapped;

fn checked_pml1_entry(pml_base: u64, virt: *mut u8, _crit: &Critical) -> Result<*mut PmlEntry, NotMapped> {
    critical::section(|| {
        unsafe {
            let virt = virt as u64;

            let pml4_ent = pml4_entry(pml_base, virt);
            let pml3_ent = pml3_entry(pml_base, virt);
            let pml2_ent = pml2_entry(pml_base, virt);
            let pml1_ent = pml1_entry(pml_base, virt);

            if (*pml4_ent).0 == 0 {
                return Err(NotMapped);
            }

            if (*pml3_ent).0 == 0 {
                return Err(NotMapped);
            }

            if (*pml2_ent).0 == 0 {
                return Err(NotMapped);
            }

            if (*pml1_ent).0 == 0 {
                return Err(NotMapped);
            }

            Ok(pml1_ent)
        }
    })
}

pub fn entry(virt: *mut u8, _crit: &Critical) -> Result<&PmlEntry, NotMapped> {
    let entry = checked_pml1_entry(CURRENT_PML, virt, _crit)?;

    // Safety(UNSAFE): ref lifetime is tied to critical section lifetime.
    // This could result in bad memory access if the page tables are mutated
    // by the kernel while this critical section is held, but importantly at
    // least protects us from concurrent modification by other processes.
    unsafe { Ok(&*entry) }
}

pub unsafe fn unmap(virt: *mut u8) -> Result<(), NotMapped> {
    let crit = critical::begin();

    let pml1_ent = checked_pml1_entry(CURRENT_PML, virt, &crit)?;

    match (*pml1_ent).raw_phys() {
        Some(raw_phys) => {
            // ensure we decrement the ref count of the physical page:
            Phys::from_raw(raw_phys);
            *pml1_ent = PmlEntry(0);
            invlpg(virt as *mut u8);
            Ok(())
        }
        None => {
            Err(NotMapped)
        }
    }
}

pub unsafe fn modify(virt: *mut u8, flags: PageFlags) -> Result<(), NotMapped> {
    let crit = critical::begin();

    let pml1_ent = checked_pml1_entry(CURRENT_PML, virt, &crit)?;
    (*pml1_ent).set_flags(flags);

    Ok(())
}


pub fn virt_to_phys(virt: *mut u8) -> Result<Phys, NotMapped> {
    let crit = critical::begin();

    unsafe {
        let pml1_ent = checked_pml1_entry(CURRENT_PML, virt, &crit)?;

        (*pml1_ent).raw_phys()
            .map(|raw_phys| Phys::new(raw_phys))
            .ok_or(NotMapped)
    }
}
