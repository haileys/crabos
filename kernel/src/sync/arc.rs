use core::ops::Deref;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::mem::kalloc;
use crate::mem::MemoryExhausted;

struct ArcObject<T> {
    ref_count: AtomicU64,
    object: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcObject<T>>,
}

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

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().object }
    }
}

impl<T> Drop for Arc<T> {
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
                kalloc::free(self.ptr);
            }
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        unsafe { self.ptr.as_ref() }
            .ref_count
            .fetch_add(1, Ordering::SeqCst);

        Arc { ptr: self.ptr }
    }
}
