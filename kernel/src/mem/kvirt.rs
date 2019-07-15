use crate::sync::Mutex;
use crate::mem::phys::{self, MemoryExhausted};
use crate::mem::page::{self, PageFlags, MapError, PAGE_SIZE};

use core::ptr::{self, NonNull};

extern { static _end: u8; }

static ALLOCATOR: PageAllocator = unsafe { PageAllocator::new(&_end as *const u8 as *mut u8) };

pub fn alloc() -> Result<NonNull<u8>, MemoryExhausted> {
    ALLOCATOR.alloc()
}

pub unsafe fn free(page: NonNull<u8>) {
    ALLOCATOR.free(page)
}

struct PageAllocator {
    inner: Mutex<PageAllocatorInner>,
}

struct PageAllocatorInner {
    free_page: Option<NonNull<FreePage>>,
    ptr: *mut u8,
}

struct FreePage {
    next: Option<NonNull<FreePage>>,
}

impl PageAllocator {
    pub const unsafe fn new(ptr: *mut u8) -> Self {
        PageAllocator {
            inner: Mutex::new(PageAllocatorInner {
                free_page: None,
                ptr: ptr,
            })
        }
    }

    pub fn alloc(&self) -> Result<NonNull<u8>, MemoryExhausted> {
        crate::println!("PageAllocator::alloc!");

        {
            let mut inner = self.inner.lock();

            if let Some(mut page) = inner.free_page.take() {
                unsafe {
                    inner.free_page = page.as_mut().next.take();
                    ptr::write_bytes(page.as_ptr(), 0, PAGE_SIZE);
                    return Ok(page.cast::<u8>());
                }
            }
        }

        let phys = phys::alloc()?;

        unsafe {
            let ptr = {
                let mut inner = self.inner.lock();
                let ptr = inner.ptr;
                inner.ptr = inner.ptr.add(PAGE_SIZE);
                ptr
            };

            match page::map(phys, ptr, PageFlags::PRESENT | PageFlags::WRITE) {
                Ok(()) => {}
                Err(MapError::CannotAllocatePageTable) => return Err(MemoryExhausted),
                Err(MapError::AlreadyMapped) => panic!("MapError::AlreadyMapped in PageAllocator::allocate"),
            }

            Ok(NonNull::new_unchecked(ptr))
        }
    }

    pub unsafe fn free(&self, page: NonNull<u8>) {
        let mut inner = self.inner.lock();

        let page = page.cast::<FreePage>();

        let link = FreePage { next: inner.free_page.take() };
        ptr::write(page.as_ptr(), link);

        inner.free_page = Some(page);
    }
}
