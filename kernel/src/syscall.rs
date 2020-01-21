use core::convert::TryInto;

use bitflags::bitflags;

use crate::{critical, println};
use crate::interrupt::{TrapFrame, Registers};
use crate::mem::user::{self, PageRange};
use crate::mem::page::{self, PageFlags, MapError};
use crate::mem::phys::{self};
use interface::{OK, Syscall, SysError, SysResult};

pub async fn dispatch(frame: &mut TrapFrame) {
    let result = dispatch0(&mut frame.regs).await;

    frame.regs.rax = match result {
        Ok(()) => OK,
        Err(e) => e.into(),
    };
}

async fn dispatch0(regs: &mut Registers) -> SysResult<()> {
    let syscall = regs.rax
        .try_into()
        .map_err(|()| SysError::BadSyscall)?;

    match syscall {
        Syscall::AllocPage => alloc_page(regs.rdi, regs.rsi, regs.rdx),
        Syscall::ReleasePage => release_page(regs.rdi, regs.rsi),
        Syscall::ModifyPage => modify_page(regs.rdi, regs.rsi, regs.rdx)
    }
}

bitflags! {
    pub struct UserPageFlags: u64 {
        const WRITE = 0x02;
    }
}

impl From<UserPageFlags> for PageFlags {
    fn from(user_flags: UserPageFlags) -> PageFlags {
        // UserPageFlags implies PRESENT and USER:
        let mut flags = PageFlags::PRESENT | PageFlags::USER;

        if user_flags.contains(UserPageFlags::WRITE) {
            flags.insert(PageFlags::WRITE);
        }

        flags
    }
}

fn alloc_page(virtual_addr: u64, page_count: u64, flags: u64) -> SysResult<()> {
    println!("SYSCALL alloc_page");

    let crit = critical::begin();

    let page_range = PageRange::new(virtual_addr, page_count)?;
    user::validate_available(&page_range, &crit)?;

    let flags = UserPageFlags::from_bits(flags)
        .ok_or(SysError::IllegalValue)?;

    let flags = PageFlags::from(flags);

    for addr in page_range.pages() {
        // TOOD - handle erroring here leaving previously allocated pages mapped
        let phys = phys::alloc()
            .map_err(|_| SysError::MemoryExhausted)?;

        let addr = addr as *mut u8;

        // Safety: we validated that this will not violate kernel memory safety
        // We do not guarantee user space memory safety
        unsafe {
            println!("mapping {:?} -> {:?}", addr, phys);

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

fn release_page(virtual_addr: u64, page_count: u64) -> SysResult<()> {
    println!("SYSCALL release_page");

    let crit = critical::begin();

    let page_range = PageRange::new(virtual_addr, page_count)?;
    user::validate_map(&page_range, PageFlags::empty(), &crit)?;

    for addr in page_range.pages() {
        let addr = addr as *mut u8;

        // Safety: we validated that this will not violate kernel memory safety
        // We do not guarantee user space memory safety
        unsafe {
            println!("releasing {:?}", addr);

            page::unmap(addr)
                .expect("release_page: NotMapped error should never happen");
        }
    }

    Ok(())
}

fn modify_page(virtual_addr: u64, page_count: u64, flags: u64) -> SysResult<()> {
    println!("SYSCALL release_page");

    let crit = critical::begin();

    let page_range = PageRange::new(virtual_addr, page_count)?;
    user::validate_map(&page_range, PageFlags::empty(), &crit)?;

    let flags = UserPageFlags::from_bits(flags)
        .ok_or(SysError::IllegalValue)?;

    let flags = PageFlags::from(flags);

    for addr in page_range.pages() {
        let addr = addr as *mut u8;

        // Safety: we validated that this will not violate kernel memory safety
        // We do not guarantee user space memory safety
        unsafe {
            println!("releasing {:?}", addr);

            page::modify(addr, flags)
                .expect("modify_page: NotMapped error should never happen");
        }
    }

    Ok(())
}
