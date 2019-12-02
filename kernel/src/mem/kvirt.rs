use core::ops::{Deref, DerefMut};

use crate::mem::MemoryExhausted;
use crate::mem::page::{self, PageFlags, MapError, PAGE_SIZE};
use crate::mem::phys;
use crate::sync::Mutex;

use core::ptr::{self, NonNull};

extern { static _end: u8; }

static ALLOCATOR: PageAllocator = unsafe { PageAllocator::new(&_end as *const u8 as *mut u8) };

pub unsafe trait PageSized {}

unsafe impl PageSized for u8 {}

pub fn alloc_page<T: PageSized>() -> Result<NonNull<T>, MemoryExhausted> {
    ALLOCATOR.alloc().map(NonNull::cast)
}

pub unsafe fn free_page<T: PageSized>(page: NonNull<T>) {
    ALLOCATOR.free(page.cast())
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

                    // cast page to u8 to simplify write_bytes:
                    let page = page.cast::<u8>();
                    ptr::write_bytes(page.as_ptr(), 0, PAGE_SIZE);
                    return Ok(page);
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

pub struct PageBox<T: PageSized> {
    page: NonNull<T>,
}

impl<T: PageSized> PageBox<T> {
    pub fn new(value: T) -> Result<Self, MemoryExhausted> {
        let page = alloc_page()?;
        unsafe { ptr::write(page.as_ptr(), value); }
        Ok(PageBox { page })
    }
}

impl<T: PageSized> Drop for PageBox<T> {
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(self.page.as_ptr()); }
    }
}

impl<T: PageSized> Deref for PageBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.page.as_ref() }
    }
}

impl<T: PageSized> DerefMut for PageBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.page.as_mut() }
    }
}
