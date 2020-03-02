use core::{mem, slice};

use crate::mem::page::{self, PAGE_SIZE, PageFlags};
use crate::critical::Critical;
use interface::{SysResult, SysError};

const MAX_USER_ADDR: u64 = 0x0000800000000000; // exclusive max

pub fn validate_page_align(addr: u64) -> SysResult<()> {
    if (addr & (PAGE_SIZE as u64 - 1)) != 0 {
        return Err(SysError::BadPointer);
    }

    Ok(())
}

#[derive(Debug)]
pub struct PageRange {
    base_page: u64,
    page_count: usize,
}

impl PageRange {
    /// Creates a new PageRange representing that validation has been performed.
    /// `base_page` must be a page-aligned address.
    pub fn new(base_page: u64, page_count: u64) -> Result<PageRange, SysError> {
        let byte_len = page_count.checked_mul(PAGE_SIZE as u64)
            .ok_or(SysError::MemoryExhausted)?;

        validate_page_align(base_page)?;

        if base_page > MAX_USER_ADDR {
            return Err(SysError::BadPointer);
        }

        if base_page + byte_len > MAX_USER_ADDR {
            return Err(SysError::BadPointer);
        }

        Ok(PageRange {
            base_page,
            page_count: page_count as usize,
        })
    }

    /// Creates the smallest PageRange that contains the given address range.
    /// The address range does not have to be page-aligned.
    pub fn containing(start_addr: u64, byte_len: u64) -> Result<PageRange, SysError> {
        let first_page_offset = start_addr % PAGE_SIZE as u64;

        // never underflows:
        let base_page = start_addr - first_page_offset;

        // extend byte_len correspondingly:
        let byte_len = byte_len.checked_add(first_page_offset)
            .ok_or(SysError::MemoryExhausted)?;

        let last_page_extent = byte_len % PAGE_SIZE as u64;

        let last_page_padding = if last_page_extent == 0 {
            0
        } else {
            PAGE_SIZE as u64 - last_page_extent
        };

        // extend byte_len again:
        let byte_len = byte_len.checked_add(last_page_padding)
            .ok_or(SysError::MemoryExhausted)?;

        let page_count = byte_len / PAGE_SIZE as u64;

        PageRange::new(base_page, page_count)
    }

    pub fn pages(&self) -> impl Iterator<Item = u64> {
        let base_page = self.base_page;

        (0..self.page_count)
            .map(|index| index * PAGE_SIZE)
            .map(move |offset| base_page + offset as u64)
    }
}

pub fn validate_map(page_range: &PageRange, flags: PageFlags, crit: &Critical)
    -> SysResult<()>
{
    for addr in page_range.pages() {
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

pub fn validate_available(page_range: &PageRange, crit: &Critical) -> SysResult<()> {
    for addr in page_range.pages() {
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
    let page_range = PageRange::containing(addr, len)?;
    validate_map(&page_range, PageFlags::empty(), crit)
}

/// Borrows a slice from user space. `len` is the number of elements, not the
/// number of bytes.
pub fn borrow_slice<T>(addr: u64, len: u64, crit: &Critical) -> SysResult<&[T]> {
    let byte_len = len.checked_mul(mem::size_of::<T>() as u64)
        .ok_or(SysError::BadPointer)?;

    validate_read(addr, byte_len, crit)?;

    // Safety(UNSAFE): ref lifetime is tied to critical section lifetime.
    // This could result in bad memory access if the page tables are mutated
    // by the kernel while this critical section is held, but importantly at
    // least protects us from concurrent modification by other processes.
    Ok(unsafe { slice::from_raw_parts(addr as *const T, len as usize) })
}

#[allow(unused)]
pub fn borrow<T>(addr: u64, crit: &Critical) -> SysResult<&T> {
    let slice = borrow_slice(addr, mem::size_of::<T>() as u64, crit)?;

    Ok(&slice[0])
}
