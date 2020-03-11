use core::convert::TryInto;

use interface::{SysResult, SysError, Syscall};
use interface::ERR_FLAG;

use crate::Handle;

#[repr(transparent)]
pub struct SyscallResult(u64);

pub unsafe trait FromSysOk {
    fn from_sys_ok(raw: u64) -> Self;
}

unsafe impl FromSysOk for usize {
    fn from_sys_ok(raw: u64) -> Self {
        // TODO: gate this on 64 bit arch
        raw as usize
    }
}

unsafe impl FromSysOk for u64 {
    fn from_sys_ok(raw: u64) -> Self {
        raw
    }
}

unsafe impl FromSysOk for Handle {
    fn from_sys_ok(raw: u64) -> Self {
        unsafe { Handle::from_raw(raw) }
    }
}

impl<T> From<SyscallResult> for Result<T, SysError> where T: FromSysOk {
    fn from(raw: SyscallResult) -> Self {
        if (raw.0 & ERR_FLAG) == 0 {
            Ok(T::from_sys_ok(raw.0))
        } else {
            Err(raw.0.try_into().expect("SysError from u64"))
        }
    }
}

unsafe fn syscall0(vector: Syscall) -> SyscallResult {
    let ret: SyscallResult;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64)
    :: "intel");

    ret
}

unsafe fn syscall1(vector: Syscall, a: u64) -> SyscallResult {
    let ret: SyscallResult;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64),
        "{rdi}"(a)
    :: "intel");

    ret
}

unsafe fn syscall2(vector: Syscall, a: u64, b: u64) -> SyscallResult {
    let ret: SyscallResult;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64),
        "{rdi}"(a),
        "{rsi}"(b)
    :: "intel");

    ret
}

unsafe fn syscall3(vector: Syscall, a: u64, b: u64, c: u64) -> SyscallResult {
    let ret: SyscallResult;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64),
        "{rdi}"(a),
        "{rsi}"(b),
        "{rdx}"(c)
    :: "intel");

    ret
}

unsafe fn syscall4(vector: Syscall, a: u64, b: u64, c: u64, d: u64) -> SyscallResult {
    let ret: SyscallResult;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64),
        "{rdi}"(a),
        "{rsi}"(b),
        "{rdx}"(c),
        "{rcx}"(d)
    :: "intel");

    ret
}

#[export_name = "syscall_alloc_page"]
pub unsafe extern "C" fn alloc_page(base_addr: *mut u8, page_count: u64, flags: u64) -> SyscallResult {
    syscall3(Syscall::AllocPage, base_addr as u64, page_count, flags)
}

#[export_name = "syscall_release_page"]
pub unsafe extern "C" fn release_page(base_addr: *mut u8, page_count: u64) -> SyscallResult {
    syscall2(Syscall::ReleasePage, base_addr as u64, page_count)
}

#[export_name = "syscall_modify_page"]
pub unsafe extern "C" fn modify_page(base_addr: *mut u8, page_count: u64, flags: u64) -> SyscallResult {
    syscall3(Syscall::ModifyPage, base_addr as u64, page_count, flags)
}

#[export_name = "syscall_map_physical_memory"]
pub unsafe extern "C" fn map_physical_memory(base_addr: *mut u8, physical_addr: u64, page_count: u64, flags: u64) -> SyscallResult {
    syscall4(Syscall::MapPhysicalMemory, base_addr as u64, physical_addr, page_count, flags)
}

#[export_name = "syscall_clone_handle"]
pub unsafe extern "C" fn clone_handle(handle: u64) -> SyscallResult  {
    syscall1(Syscall::CloneHandle, handle)
}

#[export_name = "syscall_release_handle"]
pub unsafe extern "C" fn release_handle(handle: u64) -> SyscallResult  {
    syscall1(Syscall::ReleaseHandle, handle)
}

#[export_name = "syscall_create_page_context"]
pub unsafe extern "C" fn create_page_context() -> SyscallResult {
    syscall0(Syscall::CreatePageContext)
}

#[export_name = "syscall_debug"]
pub unsafe extern "C" fn debug() -> SyscallResult {
    syscall0(Syscall::Debug)
}

#[export_name = "syscall_set_page_context"]
pub unsafe extern "C" fn set_page_context(page_ctx: u64) -> SyscallResult {
    syscall1(Syscall::SetPageContext, page_ctx)
}

#[export_name = "syscall_get_page_context"]
pub unsafe extern "C" fn get_page_context() -> SyscallResult {
    syscall0(Syscall::GetPageContext)
}

#[export_name = "syscall_create_task"]
pub unsafe extern "C" fn create_task(page_ctx: u64, rip: u64, rsp: u64) -> SyscallResult {
    syscall3(Syscall::CreateTask, page_ctx, rip, rsp)
}

#[export_name = "syscall_exit"]
pub unsafe extern "C" fn exit(status: u64) -> SyscallResult {
    syscall1(Syscall::Exit, status)
}

#[export_name = "syscall_read_file"]
pub unsafe extern "C" fn read_file(file: u64, buff: *mut u8, buff_len: u64) -> SyscallResult {
    syscall3(Syscall::ReadFile, file, buff as u64, buff_len)
}

#[export_name = "syscall_write_file"]
pub unsafe extern "C" fn write_file(file: u64, buff: *const u8, buff_len: u64) -> SyscallResult {
    syscall3(Syscall::WriteFile, file, buff as u64, buff_len)
}

#[export_name = "syscall_open_file"]
pub unsafe extern "C" fn open_file(path: *const u8, path_len: u64, flags: u64) -> SyscallResult {
    syscall3(Syscall::OpenFile, path as u64, path_len, flags)
}
