use core::fmt::{self, Debug};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::critical;
use crate::mem::page::{self, PAGE_SIZE, PageFlags};
use crate::sync::Mutex;

static PHYS_REGIONS: Mutex<Option<[PhysRegion; 8]>> = Mutex::new(None);
static mut NEXT_FREE_PHYS: Option<RawPhys> = None;

const REGION_KIND_USABLE: u32 = 1;
const HIGH_MEMORY_BOUNDARY: RawPhys = RawPhys(0x100000);


#[repr(transparent)]
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct RawPhys(pub u64);

#[derive(Clone)]
pub struct Phys(u64);

impl Phys {
    /// Creates a new Phys, incrementing the reference count of the underlying
    /// physical page by one
    unsafe fn new(raw_phys: RawPhys) -> Phys {
        // TODO increment ref count
        Phys(raw_phys.0)
    }

    /// Consumes the Phys, returning the raw address of the physical page. This
    /// method does not affect the reference count of the underlying physical
    /// page, so care must be taken to avoid leaks.
    pub fn into_raw(self) -> RawPhys {
        RawPhys(self.0)
    }

    /// Constructs a Phys from a raw address returned by `into_raw`. This
    /// function is the dual of into_raw. This function does not affect the
    /// reference count of the underlying physical page, so care must be taken
    /// to only call this once per corresponding `into_raw` call.
    pub unsafe fn from_raw(raw_phys: RawPhys) -> Phys {
        Phys(raw_phys.0)
    }
}

impl Debug for Phys {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // unsafe { asm!("xchgw %bx, %bx"); }
        write!(f, "Phys(0x{:08x})", self.0)
    }
}

#[derive(Clone)]
struct PhysRegion {
    begin: RawPhys,
    size: u64,
}

#[repr(C)]
pub struct BiosMemoryRegion {
    begin: RawPhys,
    size: u64,
    kind: u32,
    acpi_ex_attrs: u32,
}



#[derive(Debug)]
pub struct MemoryExhausted;

#[no_mangle]
pub unsafe extern "C" fn phys_init(bios_memory_map: *const BiosMemoryRegion, region_count: u16) {
    crate::println!("Initialising physical page allocator...");

    // init temp mapping
    page::temp_unmap(&critical::begin());

    let mut phys_i = 0;

    // XXX - it's important PhysRegion is not Copy to prevent bugs from
    // unintentional copies out of the mutex-guarded static. This means we're
    // unable to use the repeat array initializer syntax here:
    const NULL_PHYS_REGION: PhysRegion = PhysRegion { begin: RawPhys(0), size: 0 };
    let mut phys_regions = [
        NULL_PHYS_REGION,
        NULL_PHYS_REGION,
        NULL_PHYS_REGION,
        NULL_PHYS_REGION,
        NULL_PHYS_REGION,
        NULL_PHYS_REGION,
        NULL_PHYS_REGION,
        NULL_PHYS_REGION,
    ];

    for i in 0..region_count {
        let region = &*bios_memory_map.add(i as usize);

        crate::println!("  - Memory region 0x{:016x}, length 0x{:016x}", region.begin.0, region.size);
        crate::println!("    type {}, acpi ex attrs 0x{:08x}", region.kind, region.acpi_ex_attrs);

        if region.kind != REGION_KIND_USABLE {
            continue;
        }

        let region_begin = region.begin;
        let region_end = RawPhys(region.begin.0 + region.size);

        if region_end < HIGH_MEMORY_BOUNDARY {
            continue;
        }

        let region_begin = if region_begin < HIGH_MEMORY_BOUNDARY {
            HIGH_MEMORY_BOUNDARY
        } else {
            region_begin
        };

        let region_size = region_end.0 - region_begin.0;

        crate::println!("    - registering as region #{}", phys_i);
        phys_regions[phys_i].begin = region_begin;
        phys_regions[phys_i].size = region_size;
        phys_i += 1;

        if phys_i == phys_regions.len() {
            break;
        }
    }

    let mibibytes = phys_regions.iter().map(|reg| reg.size).sum::<u64>() / 1024 / 1024;
    crate::println!("  {} MiB free", mibibytes);

    *PHYS_REGIONS.lock() = Some(phys_regions.clone());

    crate::println!();
}

unsafe fn zero_phys(phys: RawPhys) {
    let crit = critical::begin();

    let mapped = page::temp_map::<u64>(phys, &crit);
    ptr::write_bytes(mapped, 0, PAGE_SIZE / mem::size_of::<u64>());
    page::temp_unmap(&crit);
}

fn alloc_freelist() -> Option<Phys> {
    let crit = critical::begin();

    unsafe {
        if let Some(phys) = NEXT_FREE_PHYS.take() {
            let phys = Phys::new(phys);

            let mapped = page::temp_map::<Option<RawPhys>>(RawPhys(phys.0), &crit);

            // pull linked next free phys out:
            NEXT_FREE_PHYS = (*mapped).take();

            // zero page before returning:
            ptr::write_bytes(mapped as *mut u64, 0, PAGE_SIZE / mem::size_of::<u64>());

            page::temp_unmap(&crit);
            Some(phys)
        } else {
            None
        }
    }
}

fn alloc_new(regions: &mut [PhysRegion]) -> Result<Phys, MemoryExhausted> {
    for region in regions {
        if region.size == 0 {
            continue;
        }

        let raw_phys = region.begin;
        region.begin.0 += PAGE_SIZE as u64;
        region.size -= PAGE_SIZE as u64;

        let phys = unsafe { Phys::new(raw_phys) };

        unsafe {
            let crit = critical::begin();

            let mapped = page::temp_map::<u64>(raw_phys, &crit);
            ptr::write_bytes(mapped, 0, PAGE_SIZE / mem::size_of::<u64>());
            page::temp_unmap(&crit);
        }

        return Ok(phys);
    }

    Err(MemoryExhausted)
}

pub fn alloc() -> Result<Phys, MemoryExhausted> {
    if let Some(page) = alloc_freelist() {
        return Ok(page);
    }

    alloc_new(PHYS_REGIONS.lock()
        .as_mut()
        .expect("PHYS_REGIONS is None in phys::alloc"))
}
