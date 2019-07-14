use core::convert::TryInto;

use bitflags::bitflags;

use crate::critical;
use crate::interrupt::{TrapFrame, Registers};
use crate::mem::user;
use crate::mem::page::{self, PAGE_SIZE, PageFlags, MapError};
use crate::mem::phys::{self};
use interface::{OK, Syscall, SysError, SysResult};

pub fn dispatch(frame: &mut TrapFrame) {
    let result = dispatch0(&mut frame.regs);

    frame.regs.rax = match result {
        Ok(()) => OK,
        Err(e) => e.into(),
    };
}

fn dispatch0(regs: &mut Registers) -> SysResult<()> {
    let syscall = regs.rax
        .try_into()
        .map_err(|()| SysError::BadSyscall)?;

    match syscall {
        Syscall::AllocPage => alloc_page(regs.rdi, regs.rsi, regs.rdx),
    }
}

bitflags! {
    pub struct UserPageFlags: u64 {
        const WRITE = 0x02;
    }
}

impl From<UserPageFlags> for PageFlags {
    fn from(user_flags: UserPageFlags) -> PageFlags {
        // UserPageFlags implies PRESENT:
        let mut flags = PageFlags::PRESENT;

        if user_flags.contains(UserPageFlags::WRITE) {
            flags.insert(PageFlags::WRITE);
        }

        flags
    }
}

fn alloc_page(virtual_addr: u64, page_count: u64, flags: u64) -> SysResult<()> {
    let crit = critical::begin();

    user::validate_page_align(virtual_addr)?;
    user::validate_available(virtual_addr, page_count * PAGE_SIZE as u64, &crit)?;

    let flags = UserPageFlags::from_bits(flags)
        .ok_or(SysError::IllegalValue)?;

    let flags = PageFlags::from(flags);

    crate::println!("alloc_page!");

    for i in 0..page_count {
        let phys = phys::alloc()
            .map_err(|_| SysError::MemoryExhausted)?;

        let addr = (virtual_addr + i * PAGE_SIZE as u64) as *mut u8;

        // Safety: we validated that this will not violate kernel memory safety
        // We do not guarantee user space memory safety
        unsafe {
            crate::println!("mapping {:?} -> {:?}", addr, phys);

            page::map(phys, addr, flags)
                .map_err(|e| match e {
                    MapError::AlreadyMapped => {
                        // we validate that the requested pages are available to
                        // be mapped earlier
                        panic!("alloc_page: AlreadyMapped error should never happen")
                    }
                    MapError::CannotAllocatePageTable => {
                        SysError::MemoryExhausted
                    }
                })?;
        }
    }

    Ok(())
}
