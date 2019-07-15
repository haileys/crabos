use core::alloc::{AllocErr, Layout};
use core::ptr::NonNull;

pub type AllocResult<T> = Result<T, AllocErr>;

// This is the same interface Alloc exposes, but without a self type.
pub unsafe trait GlobalAlloc {
    unsafe fn alloc(layout: Layout) -> Result<NonNull<u8>, AllocErr>;
    unsafe fn dealloc(ptr: NonNull<u8>, layout: Layout);
}
