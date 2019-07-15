use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull, Unique};
use core::alloc::{Alloc, AllocErr, Layout};

use crate::glue::GlobalAlloc;

fn alloc<T, Allocator: GlobalAlloc>(value: T) -> Result<NonNull<T>, AllocErr> {
    let ptr = unsafe { Allocator::alloc(Layout::new::<T>())?.cast() };
    unsafe { ptr::write(ptr.as_ptr(), value); }
    Ok(ptr)
}

unsafe fn free<T, Allocator: GlobalAlloc>(ptr: NonNull<T>) {
    Allocator::dealloc(ptr.cast(), Layout::new::<T>())
}

pub struct Box<T, Allocator: GlobalAlloc> {
    ptr: NonNull<T>,
    _phantom: PhantomData<Allocator>,
}

impl<T, Allocator: GlobalAlloc> Box<T, Allocator> {
    pub fn new(value: T) -> Result<Self, AllocErr> {
        Ok(Box {
            ptr: alloc::<T, Allocator>(value)?,
            _phantom: PhantomData,
        })
    }

    pub fn into_raw(b: Box<T, Allocator>) -> *mut T {
        Box::into_raw_non_null(b).as_ptr()
    }

    pub fn into_raw_non_null(b: Box<T, Allocator>) -> NonNull<T> {
        Box::into_unique(b).into()
    }

    pub fn into_unique(b: Box<T, Allocator>) -> Unique<T> {
        let mut unique = b.ptr;
        mem::forget(b);
        unsafe { Unique::new_unchecked(unique.as_mut() as *mut T) }
    }
}

impl<T, Allocator: GlobalAlloc> Drop for Box<T, Allocator> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.ptr.as_ptr());
            free::<T, Allocator>(self.ptr);
        }
    }
}

impl<T, Allocator: GlobalAlloc> Deref for Box<T, Allocator> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T, Allocator: GlobalAlloc> DerefMut for Box<T, Allocator> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}
