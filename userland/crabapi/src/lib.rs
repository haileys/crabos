#![no_std]
#![feature(asm)]
#![feature(core_panic)]
#![feature(panic_info_message)]
#![feature(start)]

pub mod fs;
pub mod io;
pub mod syscall;
pub mod task;

mod panic;

#[repr(transparent)]
pub struct Handle(u64);

impl Handle {
    pub const unsafe fn from_raw(handle: u64) -> Self {
        Handle(handle)
    }

    pub fn as_raw(&self) -> u64 {
        self.0
    }
}

impl Clone for Handle {
    fn clone(&self) -> Self {
        unsafe {
            Result::from(syscall::clone_handle(self.0))
                .expect("syscall::clone_handle")
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { syscall::release_handle(self.0); }
    }
}
