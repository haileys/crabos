use crate::critical::Critical;
use crate::mem::phys::Phys;

#[repr(transparent)]
struct Entry(u32);

pub const PAGE_SIZE: usize = 0x1000;

enum PageFlag {
    Present     = 0x001,
    Write       = 0x002,
    User        = 0x004,
}

const PAGE_DIRECTORY: *mut Entry = 0xfffff000 as *mut Entry;
const PAGE_TABLES: *mut Entry = 0xffc00000 as *mut Entry;

extern "C" {
    static mut temp_page: u8;
}

pub fn invlpg(virt: *const ()) {
    unsafe { asm!("invlpg ($0)" :: "r"(virt) : "memory" : "volatile"); }
}

// the existence of a reference to CriticalLock proves we're in a critical
// section:
pub unsafe fn temp_map<T>(phys: Phys, _critical: &Critical) -> *mut T {
    let virt = &mut temp_page as *mut u8 as *mut ();

    let entry = PAGE_TABLES.add(virt as usize >> 12) as *mut Entry;
    *entry = Entry(phys.0 | PageFlag::Present as u32 | PageFlag::Write as u32);
    invlpg(virt);

    virt as *mut T
}

pub unsafe fn temp_unmap(_critical: &Critical) {
    let virt = &mut temp_page as *mut u8 as *mut ();

    let entry = PAGE_TABLES.add(virt as usize >> 12) as *mut Entry;
    *entry = Entry(0);
    invlpg(virt);
}
