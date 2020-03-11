use core::cell::UnsafeCell;
use core::fmt::{self, Debug};
use core::future::Future;
use core::ops::{Drop, Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Poll, Waker};

use futures::future;

use crate::critical::{self, Critical};
use crate::mem::MemoryExhausted;
use crate::util::AtomicList;

pub struct AsyncMutex<T> {
    value: UnsafeCell<T>,
    locked: AtomicBool,
    wakers: AtomicList<Waker>,
}

impl<T> Debug for AsyncMutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AsyncMutex({:?})", self)
    }
}

unsafe impl<T> Sync for AsyncMutex<T> {}

impl<T> AsyncMutex<T> {
    pub const fn new(value: T) -> Self {
        AsyncMutex {
            value: UnsafeCell::new(value),
            locked: AtomicBool::new(false),
            wakers: AtomicList::new(),
        }
    }

    pub fn lock<'a>(&'a self) -> impl Future<Output = Result<AsyncMutexGuard<'a, T>, MemoryExhausted>> + 'a {
        future::poll_fn(move |ctx| {
            // always push a waker to resolve race condition between swapped
            // locked value and pushing a waker
            match self.wakers.push_front(ctx.waker().clone()) {
                Ok(()) => {}
                Err(e) => { return Poll::Ready(Err(e)); }
            }

            let previous = self.locked.swap(true, Ordering::SeqCst);

            if previous == false {
                // we got the lock!
                // just leave our waker in the list for now... it's not a
                // problem to spuriously wake up
                return Poll::Ready(Ok(AsyncMutexGuard {
                    mutex: self,
                }));
            }

            Poll::Pending
        })
    }
}

pub struct AsyncMutexGuard<'a, T> {
    mutex: &'a AsyncMutex<T>,
}

impl<'a, T> Drop for AsyncMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::SeqCst);

        for waker in self.mutex.wakers.take_iter() {
            waker.wake();
        }
    }
}

impl<'a, T> Deref for AsyncMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: we have the lock
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'a, T> DerefMut for AsyncMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: we have the lock
        unsafe { &mut *self.mutex.value.get() }
    }
}
