use core::{mem, slice};

use crate::mem::page::{self, PAGE_SIZE, PageFlags};
use crate::critical::Critical;
use interface::{SysResult, SysError};

const MAX_USER_ADDR: u64 = 0x0000800000000000; // exclusive max

fn mappable_addr(addr: u64, len: u64) -> bool {
    if addr > MAX_USER_ADDR {
        return false;
    }

    if addr.saturating_add(len) > MAX_USER_ADDR {
        return false;
    }

    true
}

pub fn validate_page_align(addr: u64) -> SysResult<()> {
    if (addr & (PAGE_SIZE as u64 - 1)) != 0 {
        return Err(SysError::BadPointer);
    }

    Ok(())
}

pub fn pages(addr: u64, len: u64)
    -> impl Iterator<Item = u64>
{
    let end = addr + len;

    // round starting address down to page boundary
    let start = addr & !(PAGE_SIZE as u64 - 1);

    (start..end).step_by(PAGE_SIZE)
}

pub fn validate_map(addr: u64, len: u64, flags: PageFlags, crit: &Critical)
    -> SysResult<()>
{
    if !mappable_addr(addr, len) {
        return Err(SysError::BadPointer);
    }

    for addr in pages(addr, len) {
        let valid = page::entry(addr as *mut u8, crit)
            .map(|entry|
                entry.flags().contains(flags))
            .unwrap_or(false);

        if !valid {
            return Err(SysError::BadPointer);
        }
    }

    Ok(())
}

pub fn validate_available(addr: u64, len: u64, crit: &Critical) -> SysResult<()> {
    if !mappable_addr(addr, len) {
        // TODO should we return BadPointer or AlreadyMapped for kernel
        // addresses in this function?
        return Err(SysError::BadPointer);
    }

    crate::println!("addr: {:x?}, len: {:x?}", addr, len);

    for addr in pages(addr, len) {
        crate::println!("validate_available checking address {:x?}", addr);
        let exists = page::entry(addr as *mut u8, crit).is_ok();

        if exists {
            return Err(SysError::AlreadyMapped);
        }
    }

    Ok(())
}

pub fn validate_read(addr: u64, len: u64, crit: &Critical) -> SysResult<()> {
    // explicitly not checking for PRESENT flag, as this prevent us from
    // faulting in pages
    validate_map(addr, len, PageFlags::empty(), crit)
}

/// Borrows a slice from user space. `len` is the number of elements, not the
/// number of bytes.
pub fn borrow_slice<T>(addr: u64, len: u64, crit: &Critical) -> SysResult<&[T]> {
    let byte_len = len.checked_mul(mem::size_of::<T> as u64)
        .ok_or(SysError::BadPointer)?;

    validate_read(addr, byte_len, crit)?;

    // Safety(UNSAFE): ref lifetime is tied to critical section lifetime.
    // This could result in bad memory access if the page tables are mutated
    // by the kernel while this critical section is held, but importantly at
    // least protects us from concurrent modification by other processes.
    Ok(unsafe { slice::from_raw_parts(addr as *const T, len as usize) })
}

pub fn borrow<T>(addr: u64, crit: &Critical) -> SysResult<&T> {
    let slice = borrow_slice(addr, mem::size_of::<T>() as u64, crit)?;

    Ok(&slice[0])
}
