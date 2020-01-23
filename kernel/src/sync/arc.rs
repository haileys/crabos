use core::alloc::Layout;
use core::marker::{Unpin, Unsize};
use core::ops::{Deref, CoerceUnsized};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::mem::kalloc;
use crate::mem::MemoryExhausted;

#[derive(Debug)]
struct ArcObject<T: ?Sized> {
    ref_count: AtomicU64,
    object: T,
}

#[derive(Debug)]
pub struct Arc<T: ?Sized> {
    ptr: NonNull<ArcObject<T>>,
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Arc<U>> for Arc<T> {}

impl<T> Unpin for Arc<T> {}

impl<T> Arc<T> {
    pub fn new(object: T) -> Result<Arc<T>, MemoryExhausted> {
        Ok(Arc {
            ptr: kalloc::alloc(ArcObject {
                ref_count: AtomicU64::new(1),
                object,
            })?,
        })
    }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().object }
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        let ref_count = unsafe { self.ptr.as_ref() }
            .ref_count
            .fetch_sub(1, Ordering::SeqCst);

        if ref_count == 0 {
            panic!("Arc::drop: ref_count underflow!");
        }

        if ref_count == 1 {
            // we're the last Arc alive
            unsafe {
                ptr::drop_in_place(self.ptr.as_ptr());
                kalloc::free_layout(Layout::for_value(&self.ptr), self.ptr.cast());
            }
        }
    }
}

impl<T: ?Sized> Clone for Arc<T> {
    fn clone(&self) -> Self {
        unsafe { self.ptr.as_ref() }
            .ref_count
            .fetch_add(1, Ordering::SeqCst);

        Arc { ptr: self.ptr }
    }
}
