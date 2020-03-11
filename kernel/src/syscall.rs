use core::convert::TryInto;

use bitflags::bitflags;
use interface::{OK, Syscall, SysError, SysResult};

use crate::interrupt::{TrapFrame, Registers};
use crate::mem::page::{self, PageFlags, MapError, PageCtx, PAGE_SIZE};
use crate::mem::phys::{self, Phys, RawPhys};
use crate::mem::user::{self, PageRange};
use crate::object::{self, Handle, Object, ObjectKind, ObjectRef};
use crate::fs::vfs::File;
use crate::task;
use crate::{critical, println};

mod args;
use args::UserArg;

pub async fn dispatch(frame: &mut TrapFrame) {
    let result = dispatch0(&mut frame.regs).await;

    frame.regs.rax = match result {
        Ok(u) => u,
        Err(e) => e as u64,
    };
}

async fn dispatch0(regs: &mut Registers) -> SyscallReturn {
    let syscall = regs.rax
        .try_into()
        .map_err(|()| SysError::BadSyscall)?;

    match syscall {
        Syscall::AllocPage => alloc_page(regs.rdi, regs.rsi, regs.rdx),
        Syscall::ReleasePage => release_page(regs.rdi, regs.rsi),
        Syscall::ModifyPage => modify_page(regs.rdi, regs.rsi, regs.rdx),
        Syscall::CloneHandle => clone_handle(UserArg::from_reg(regs.rdi)?),
        Syscall::ReleaseHandle => release_handle(UserArg::from_reg(regs.rdi)?),
        Syscall::CreatePageContext => create_page_context(),
        Syscall::Debug => debug(regs),
        Syscall::SetPageContext => set_page_context(UserArg::from_reg(regs.rdi)?),
        Syscall::GetPageContext => get_page_context(),
        Syscall::CreateTask => create_task(UserArg::from_reg(regs.rdi)?, regs.rsi, regs.rdx),
        Syscall::Exit => exit(UserArg::from_reg(regs.rdi)?),
        Syscall::MapPhysicalMemory => map_physical_memory(regs.rdi, regs.rsi, regs.rdx, regs.rcx),
        Syscall::ReadFile => read_file(UserArg::from_reg(regs.rdi)?, regs.rsi, regs.rdx).await,
        Syscall::WriteFile => write_file(UserArg::from_reg(regs.rdi)?, regs.rsi, regs.rdx).await,
        Syscall::OpenFile => open_file(regs.rdi, regs.rsi, regs.rdx).await,
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

type SyscallReturn = SysResult<u64>;

fn alloc_page(virtual_addr: u64, page_count: u64, flags: u64) -> SyscallReturn {
    println!("SYSCALL alloc_page(virt = {:x?}, count = {:x?}, flags = {:x?})",
        virtual_addr,  page_count, flags);

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

    Ok(OK)
}

fn release_page(virtual_addr: u64, page_count: u64) -> SyscallReturn {
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

    Ok(OK)
}

fn modify_page(virtual_addr: u64, page_count: u64, flags: u64) -> SyscallReturn {
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

    Ok(OK)
}

fn map_physical_memory(virtual_addr: u64, physical_addr: u64, page_count: u64, flags: u64)
    -> SyscallReturn
{
    println!("SYSCALL map_physical_memory(virt = {:x?}, phys = {:x?}, count = {:x?}, flags = {:x?})",
        virtual_addr, physical_addr, page_count, flags);

    // TODO - insert capabilities check here. calling process must have DRIVER caps

    let crit = critical::begin();

    let page_range = PageRange::new(virtual_addr, page_count)?;
    user::validate_available(&page_range, &crit)?;

    let flags = UserPageFlags::from_bits(flags)
        .ok_or(SysError::IllegalValue)?;

    let flags = PageFlags::from(flags);

    for (addr, phys) in page_range.pages().zip((physical_addr..).step_by(PAGE_SIZE)) {
        let addr = addr as *mut u8;

        // Safety: This may violate kernel memory safety, but given that the
        // calling process has driver privileges, all bets are off anyway. We
        // trust it to do the right thing.
        unsafe {
            let phys = Phys::new(RawPhys(phys));

            page::map(phys, addr, flags)
                // we've already validated mapping above, so this should never fail:
                .expect("page::map in map_physical_memory");
        }
    }

    Ok(OK)
}

fn clone_handle(handle: Handle) -> SyscallReturn  {
    let object_ref = object::get(task::current(), handle)
        .ok_or(SysError::BadHandle)?;

    Ok(object::put(task::current(), object_ref)?.into_u64())
}


fn release_handle(handle: Handle) -> SyscallReturn  {
    object::release(task::current(), handle)
        .map_err(|_| SysError::BadHandle)?;

    Ok(OK)
}

fn create_page_context() -> SyscallReturn {
    let page_ctx = PageCtx::new()
        .map_err(|_| SysError::MemoryExhausted)?;

    let obj = Object::new(ObjectKind::PageCtx(page_ctx))
        .map_err(|_| SysError::MemoryExhausted)?;

    Ok(object::put(task::current(), obj)?.into_u64())
}

fn debug(regs: &mut Registers) -> SyscallReturn {
    println!("{:#x?}", regs);
    Ok(OK)
}

fn set_page_context(page_ctx: Handle) -> SyscallReturn {
    let page_ctx = object::get(task::current(), page_ctx)
        .ok_or(SysError::BadHandle)?
        .downcast::<PageCtx>()?
        .object()
        .clone();

    // TODO we need to set task's page ctx too?
    unsafe { page::set_ctx(page_ctx); }

    Ok(OK)
}

fn get_page_context() -> SyscallReturn {
    let page_ctx = task::get_page_ctx();

    Ok(object::put(task::current(), page_ctx.as_dyn())?.into_u64())
}

fn create_task(page_ctx: Handle, rip: u64, rsp: u64) -> SyscallReturn {
    let page_ctx = object::get(task::current(), page_ctx)
        .ok_or(SysError::BadHandle)?
        .downcast::<PageCtx>()?
        .clone();

    let filesystem = task::get_filesystem();

    task::spawn(page_ctx, filesystem, |task| async move {
        task.setup(TrapFrame::new(rip, rsp)).run_loop().await
    })?;

    Ok(OK)
}

fn exit(_status: u64) -> SyscallReturn {
    // TODO implement
    panic!("process exited!")
}

async fn read_file(file: Handle, buf: u64, nbyte: u64) -> SyscallReturn {
    let file = object::get(task::current(), file)
        .ok_or(SysError::BadHandle)?
        .downcast::<File>()?;

    let crit = critical::begin();
    let buf = user::borrow_slice_mut::<u8>(buf, nbyte, &crit)?;

    file.object()
        .read(buf)
        .await
        .map(|sz| sz as u64)
}

async fn write_file(file: Handle, buf: u64, nbyte: u64) -> SyscallReturn {
    let file = object::get(task::current(), file)
        .ok_or(SysError::BadHandle)?
        .downcast::<File>()?;

    let crit = critical::begin();
    let buf = user::borrow_slice::<u8>(buf, nbyte, &crit)?;

    file.object()
        .write(buf)
        .await
        .map(|sz| sz as u64)
}

bitflags! {
    pub struct OpenFileFlags: u64 {
        const WRITE = 0x01;
    }
}

async fn open_file(path: u64, path_len: u64, flags: u64) -> SyscallReturn {
    crate::println!("open_file: {:x?}, {:x?}, {:x?}", path, path_len, flags);
    let crit = critical::begin();
    let path = user::borrow_slice::<u8>(path, path_len, &crit)?;

    let flags = OpenFileFlags::from_bits(flags)
        .ok_or(SysError::IllegalValue)?;

    let fs = task::get_filesystem().ok_or(SysError::NoFile)?;
    let file = ObjectRef::new(fs.open(path).await?)?;

    Ok(object::put(task::current(), file.as_dyn())?.into_u64())
}
