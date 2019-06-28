use core::ops::{Drop, Deref, DerefMut};
use core::cell::UnsafeCell;
use crate::critical::{self, Critical};

pub struct Mutex<T> {
    inner: UnsafeCell<MutexInner<T>>,
}

unsafe impl<T> Sync for Mutex<T> {}

struct MutexInner<T> {
    value: T,
    locked: bool,
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Mutex {
            inner: UnsafeCell::new(MutexInner {
                value: value,
                locked: false,
            })
        }
    }

    pub fn locked(&self, _critical: &Critical) -> bool {
        let inner = unsafe {
            // ref to Critical proves we're in a critical section, so this is
            // safe:
            &*self.inner.get()
        };

        inner.locked
    }

    pub fn lock<'a>(&'a self) -> MutexGuard<'a, T> {
        let critical = critical::begin();

        if self.locked(&critical) {
            panic!("recursive mutex lock!");
        }

        let inner = unsafe {
            // we are in critical section and have checked that the mutex is not
            // already locked, so this is safe:
            &mut *self.inner.get()
        };

        inner.locked = true;

        MutexGuard {
            _critical: critical,
            inner: inner,
        }
    }
}

pub struct MutexGuard<'a, T> {
    _critical: Critical,
    inner: &'a mut MutexInner<T>,
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.inner.locked = false;
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner.value
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner.value
    }
}
