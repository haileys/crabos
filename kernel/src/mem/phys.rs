use core::fmt::{self, Debug};
use core::ptr;
use crate::critical;
use crate::mem::page::{self, PAGE_SIZE};

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

#[derive(Clone, Copy)]
struct PhysRegion {
    begin: RawPhys,
    size: u64,
}

static mut PHYS_REGIONS: [PhysRegion; 8] = [PhysRegion { begin: RawPhys(0), size: 0 }; 8];
static mut NEXT_FREE_PHYS: Option<RawPhys> = None;

#[repr(C)]
pub struct BiosMemoryRegion {
    begin: RawPhys,
    size: u64,
    kind: u32,
    acpi_ex_attrs: u32,
}

const REGION_KIND_USABLE: u32 = 1;

const HIGH_MEMORY_BOUNDARY: RawPhys = RawPhys(0x100000);

#[derive(Debug)]
pub struct MemoryExhausted;

#[no_mangle]
pub unsafe extern "C" fn phys_init(bios_memory_map: *const BiosMemoryRegion, region_count: u16) {
    crate::println!("Initialising physical page allocator...");

    let mut phys_i = 0;

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
        PHYS_REGIONS[phys_i].begin = region_begin;
        PHYS_REGIONS[phys_i].size = region_size;
        phys_i += 1;

        if phys_i == PHYS_REGIONS.len() {
            break;
        }
    }

    let mibibytes = PHYS_REGIONS.iter().map(|reg| reg.size).sum::<u64>() / 1024 / 1024;
    crate::println!("  {} MiB free", mibibytes);

    crate::println!();
}

pub fn alloc() -> Result<Phys, MemoryExhausted> {
    let crit = critical::begin();

    unsafe {
        match NEXT_FREE_PHYS.take() {
            Some(raw_phys) => {
                let mapped = page::temp_map::<Option<RawPhys>>(raw_phys, &crit)
                    // this should never fail:
                    //   - a temporary mapping should not exist on entry to
                    //     this function
                    //   - the page directory entry for the temporary page
                    //     should already exist
                    .expect("page::temp_map");

                // pull linked next free phys out:
                NEXT_FREE_PHYS = (*mapped).take();

                // zero page before returning:
                ptr::write_bytes(mapped, 0, PAGE_SIZE);

                page::temp_unmap(&crit);
                Ok(Phys::new(raw_phys))
            }
            None => {
                for region in &mut PHYS_REGIONS {
                    if region.size == 0 {
                        continue;
                    }

                    let raw_phys = region.begin;
                    region.begin.0 += PAGE_SIZE as u64;
                    region.size -= PAGE_SIZE as u64;

                    let mapped = page::temp_map::<u8>(raw_phys, &crit)
                        .expect("page::temp_map");
                    ptr::write_bytes(mapped, 0, PAGE_SIZE);
                    page::temp_unmap(&crit);

                    return Ok(Phys::new(raw_phys));
                }

                Err(MemoryExhausted)
            }
        }
    }
}

impl Drop for Phys {
    fn drop(&mut self) {
        // TODO decrement ref count

        let crit = critical::begin();

        unsafe {
            let link = page::temp_map::<Option<RawPhys>>(RawPhys(self.0), &crit)
                .expect("page::temp_map");
            *link = NEXT_FREE_PHYS.take();
            page::temp_unmap(&crit);

            NEXT_FREE_PHYS = Some(RawPhys(self.0));
        }
    }
}
