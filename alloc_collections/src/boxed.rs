use core::alloc::{AllocErr, Layout};
use core::marker::{PhantomData, Unpin, Unsize};
use core::mem;
use core::ops::{Deref, DerefMut, CoerceUnsized};
use core::pin::Pin;
use core::ptr::{self, NonNull, Unique};

use crate::glue::GlobalAlloc;

fn alloc<T, Allocator: GlobalAlloc>(value: T) -> Result<NonNull<T>, AllocErr> {
    let layout = Layout::for_value(&value);
    let ptr = unsafe { Allocator::alloc(layout)?.cast() };
    unsafe { ptr::write(ptr.as_ptr(), value); }
    Ok(ptr)
}

unsafe fn free<T: ?Sized, Allocator: GlobalAlloc>(ptr: NonNull<T>) {
    let layout = Layout::for_value(ptr.as_ref());
    Allocator::dealloc(ptr.cast(), layout)
}

pub struct Box<T: ?Sized, Allocator: GlobalAlloc> {
    ptr: NonNull<T>,
    _phantom: PhantomData<Allocator>,
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, Allocator: GlobalAlloc> CoerceUnsized<Box<U, Allocator>> for Box<T, Allocator> {}

impl<T, Allocator: GlobalAlloc> Box<T, Allocator> {
    pub fn new(value: T) -> Result<Self, AllocErr> {
        Ok(Box {
            ptr: alloc::<T, Allocator>(value)?,
            _phantom: PhantomData,
        })
    }
}

impl<T, Allocator: GlobalAlloc> Unpin for Box<T, Allocator> { }

impl<T: ?Sized, Allocator: GlobalAlloc> Box<T, Allocator> {
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

impl<T: ?Sized, Allocator: GlobalAlloc> Drop for Box<T, Allocator> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.ptr.as_ptr());
            free::<T, Allocator>(self.ptr);
        }
    }
}

impl<T: ?Sized, Allocator: GlobalAlloc> Deref for Box<T, Allocator> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized, Allocator: GlobalAlloc> DerefMut for Box<T, Allocator> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}
