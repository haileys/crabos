use core::iter::Iterator;
use core::mem;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::mem::MemoryExhausted;
use crate::mem::kalloc::Box;

pub struct AtomicList<T> {
    head: AtomicPtr<Item<T>>,
}

struct Item<T> {
    next: AtomicPtr<Item<T>>,
    item: T,
}

impl<T> AtomicList<T> {
    pub const fn new() -> Self {
        AtomicList { head: AtomicPtr::new(ptr::null_mut()) }
    }

    pub fn push_front(&self, item: T) -> Result<(), MemoryExhausted> {
        let mut item = Box::into_raw(
            Box::new(Item { item, next: AtomicPtr::new(ptr::null_mut()) })
                .map_err(|_| MemoryExhausted)?);

        loop {
            // TODO - relax these SeqCst ops

            let head = self.head.load(Ordering::SeqCst);

            // Safety: item is guaranteed to exist and be owned by this function
            unsafe { (*item).next.store(head, Ordering::SeqCst) };

            let prev = self.head.compare_and_swap(head, item, Ordering::SeqCst);

            if prev == head {
                // successful update
                break;
            }
        }

        Ok(())
    }

    pub fn take_iter(&self) -> AtomicListIter<T> {
        let head = self.head.swap(ptr::null_mut(), Ordering::SeqCst);
        AtomicListIter { head: AtomicPtr::new(head) }
    }
}

pub struct AtomicListIter<T> {
    head: AtomicPtr<Item<T>>,
}

impl<T> Iterator for AtomicListIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let mut head = ptr::null_mut();
        mem::swap(self.head.get_mut(), &mut head);

        if head == ptr::null_mut() {
            return None;
        }

        let Item { next, item } = Box::into_inner(unsafe { Box::from_raw(head) });
        self.head = next;
        Some(item)
    }
}
