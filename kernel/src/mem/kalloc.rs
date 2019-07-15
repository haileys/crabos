use core::mem;
use core::ptr::{self, NonNull};

use crate::mem::{kvirt, MemoryExhausted};
use crate::mem::page::PAGE_SIZE;
use crate::println;
use crate::sync::Mutex;

static ALLOCATOR: Mutex<Allocator> = Mutex::new(Allocator::new());

pub fn alloc<T>() -> Result<NonNull<T>, MemoryExhausted> {
    ALLOCATOR.lock().alloc()
}

pub unsafe fn free<T>(ptr: NonNull<T>) {
    ALLOCATOR.lock().free(ptr)
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
            unsafe { self.free(ptr); }
        }

        Ok(self.free.take()
            .expect("alloc_uninitialized: self.free.take() should always be Some"))
    }

    pub fn alloc(&mut self) -> Result<NonNull<u8>, MemoryExhausted> {
        let ptr = self.alloc_uninitialized()?;
        unsafe { ptr::write_bytes(ptr.as_ptr(), 0, self.size); }
        println!("Allocating in size class {}: {:x?}", self.size, ptr);
        Ok(ptr)
    }

    pub unsafe fn free(&mut self, ptr: NonNull<u8>) {
        println!("Freeing in size class {}: {:x?}", self.size, ptr);

        let object = ptr.cast::<FreeObject>();

        let mut free = FreeObject { next: None };
        mem::swap(&mut self.free, &mut free);
        ptr::write(object.as_ptr(), free);

        self.free.next = Some(object);
    }
}

pub struct Allocator {
    classes: [SizeClass; 8]
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
                ]
            }
        }
    }

    fn class<T>(&mut self) -> &mut SizeClass {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        let aligned_size = size + (size % align);

        let class = self.classes.iter_mut()
            .find(|class| class.size >= aligned_size);

        match class {
            Some(class) => class,
            None => {
                panic!("no size class for size = {}, align = {}, aligned_size = {}",
                    size, align, aligned_size)
            }
        }
    }

    pub fn alloc<T>(&mut self) -> Result<NonNull<T>, MemoryExhausted> {
        self.class::<T>()
            .alloc()
            .map(NonNull::cast)
    }

    pub unsafe fn free<T>(&mut self, ptr: NonNull<T>) {
        self.class::<T>()
            .free(ptr.cast())
    }
}
