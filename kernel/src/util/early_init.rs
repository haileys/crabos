use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

const UNINIT: usize = 0;
const MIDINIT: usize = 1;
const SAFE: usize = 2;

pub struct EarlyInit<T> {
    init: AtomicUsize,
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Sync for EarlyInit<T> where T: Sync {}
unsafe impl<T> Send for EarlyInit<T> where T: Send {}

impl<T> EarlyInit<T> {
    pub const fn new() -> Self {
        EarlyInit {
            init: AtomicUsize::new(UNINIT),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn set(early_init: &Self, value: T) {
        if early_init.init.compare_and_swap(UNINIT, MIDINIT, Ordering::SeqCst) != UNINIT {
            panic!("called EarlyInit::set twice on same value!");
        }

        let maybe_uninit = unsafe { &mut *early_init.value.get() };

        unsafe { ptr::write(maybe_uninit.as_mut_ptr(), value); }

        early_init.init.store(SAFE, Ordering::SeqCst);
    }
}

impl<T> Deref for EarlyInit<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if self.init.load(Ordering::Acquire) != SAFE {
            panic!("EarlyInit not yet initialised");
        }

        unsafe { &*(*self.value.get()).as_ptr() }
    }
}
