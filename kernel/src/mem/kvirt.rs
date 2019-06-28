use crate::sync::Mutex;
use crate::mem::phys;
use crate::mem::page::{self, PageFlags};
use crate::println;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    panic!("kernel allocation failed: {:?}", layout);
}

pub struct WatermarkAllocator {
    inner: Mutex<WatermarkInner>,
}

struct WatermarkInner {
    map_end: *mut u8,
    ptr: *mut u8,
}

struct AllocError;

impl WatermarkAllocator {
    /// Safety: base *MUST* be page aligned
    pub const unsafe fn new(base: *mut u8) -> Self {
        WatermarkAllocator {
            inner: Mutex::new(WatermarkInner {
                map_end: base,
                ptr: base,
            }),
        }
    }

    fn allocate(&self, layout: Layout) -> Result<*mut u8, AllocError> {
        let mut inner = self.inner.lock();

        let align_offset = inner.ptr.align_offset(layout.align());

        if align_offset == usize::max_value() {
            return Err(AllocError);
        }

        let ptr = inner.ptr.wrapping_add(align_offset);

        let ptr_end = ptr.wrapping_add(layout.size());

        if ptr_end < ptr {
            return Err(AllocError);
        }

        // fill pages:
        while inner.map_end < ptr_end {
            let phys = phys::alloc().map_err(|_| AllocError)?;

            unsafe {
                page::map(phys, inner.map_end, PageFlags::PRESENT | PageFlags::WRITE)
                    .map_err(|_| AllocError)?;

                inner.map_end = inner.map_end.add(page::PAGE_SIZE);
            }
        }

        inner.ptr = ptr_end;
        Ok(ptr)
    }
}

unsafe impl GlobalAlloc for WatermarkAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        println!("WatermarkAllocator::alloc: {:?}", layout);

        match self.allocate(layout) {
            Ok(ptr) => ptr,
            Err(_) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        println!("WatermarkAllocator::dealloc: not deallocating");
    }
}
