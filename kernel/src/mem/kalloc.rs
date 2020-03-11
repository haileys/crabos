use core::alloc::{AllocErr, Layout};
use core::mem;
use core::ptr::{self, NonNull};

use crate::mem::{kvirt, MemoryExhausted};
use crate::mem::page::PAGE_SIZE;
use crate::sync::Mutex;

pub type Box<T> = alloc_collections::boxed::Box<T, GlobalAlloc>;

static ALLOCATOR: Mutex<Allocator> = Mutex::new(Allocator::new());

pub fn alloc<T>(value: T) -> Result<NonNull<T>, MemoryExhausted> {
    ALLOCATOR.lock().alloc(value)
}

pub unsafe fn free_layout(layout: Layout, ptr: NonNull<u8>) {
    ALLOCATOR.lock().free_layout(layout, ptr)
}

struct FreeObject {
    next: Option<NonNull<FreeObject>>,
}

impl FreeObject {
    pub fn take(&mut self) -> Option<NonNull<u8>> {
        let mut next = self.next.take()?;
        self.next = unsafe { next.as_mut().next.take() };
        Some(next.cast())
    }
}

struct SizeClass {
    size: usize,
    free: FreeObject,
}

impl SizeClass {
    /// `size` MUST be a power of two and less than or equal to PAGE_SIZE
    pub const unsafe fn new(size: usize) -> Self {
        SizeClass { size, free: FreeObject { next: None } }
    }

    fn alloc_uninitialized(&mut self) -> Result<NonNull<u8>, MemoryExhausted> {
        if let Some(ptr) = self.free.take() {
            return Ok(ptr);
        }

        let new_page = kvirt::alloc_page::<u8>()?.as_ptr();

        for offset in (0..PAGE_SIZE).step_by(self.size) {
            let ptr = unsafe { NonNull::new_unchecked(new_page.add(offset)) };
            unsafe { self.add_free(ptr); }
        }

        Ok(self.free.take()
            .expect("alloc_uninitialized: self.free.take() should always be Some"))
    }

    pub fn alloc(&mut self) -> Result<NonNull<u8>, MemoryExhausted> {
        let ptr = self.alloc_uninitialized()?;
        unsafe { ptr::write_bytes(ptr.as_ptr(), 0, self.size); }
        Ok(ptr)
    }

    unsafe fn add_free(&mut self, ptr: NonNull<u8>) {
        let object = ptr.cast::<FreeObject>();

        let mut free = FreeObject { next: None };
        mem::swap(&mut self.free, &mut free);
        ptr::write(object.as_ptr(), free);

        self.free.next = Some(object);
    }

    pub unsafe fn free(&mut self, ptr: NonNull<u8>) {
        self.add_free(ptr);
    }

    pub fn fits(&self, layout: Layout) -> bool {
        self.size >= layout.size() && self.size >= layout.align()
    }
}

pub struct Allocator {
    classes: [SizeClass; 9]
}

impl Allocator {
    pub const fn new() -> Self {
        unsafe {
            Allocator {
                classes: [
                    SizeClass::new(16),
                    SizeClass::new(32),
                    SizeClass::new(64),
                    SizeClass::new(128),
                    SizeClass::new(256),
                    SizeClass::new(512),
                    SizeClass::new(1024),
                    SizeClass::new(2048),
                    SizeClass::new(4096),
                ]
            }
        }
    }

    fn class(&mut self, layout: Layout) -> &mut SizeClass {
        let class = self.classes.iter_mut()
            .find(|class| class.fits(layout));

        match class {
            Some(class) => class,
            None => panic!("no size class for layout = {:?}", layout)
        }
    }

    pub fn alloc_layout(&mut self, layout: Layout) -> Result<NonNull<u8>, MemoryExhausted> {
        self.class(layout).alloc()
    }

    pub fn alloc<T>(&mut self, value: T) -> Result<NonNull<T>, MemoryExhausted> {
        let ptr = self.alloc_layout(Layout::new::<T>())?.cast();
        unsafe { ptr::write(ptr.as_ptr(), value); }
        Ok(ptr)
    }

    pub unsafe fn free_layout(&mut self, layout: Layout, ptr: NonNull<u8>) {
        self.class(layout).free(ptr)
    }
}

pub struct GlobalAlloc;

unsafe impl alloc_collections::glue::GlobalAlloc for GlobalAlloc {
    unsafe fn alloc(layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        ALLOCATOR.lock()
            .alloc_layout(layout)
            .map_err(|_| AllocErr)
    }

    unsafe fn dealloc(ptr: NonNull<u8>, layout: Layout) {
        ALLOCATOR.lock()
            .free_layout(layout, ptr)
    }
}
