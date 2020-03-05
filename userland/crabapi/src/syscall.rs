use interface::Syscall;

unsafe fn syscall0(vector: Syscall) -> u64 {
    let ret: u64;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64)
    :: "intel");

    ret
}

unsafe fn syscall1(vector: Syscall, a: u64) -> u64 {
    let ret: u64;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64),
        "{rdi}"(a)
    :: "intel");

    ret
}

unsafe fn syscall2(vector: Syscall, a: u64, b: u64) -> u64 {
    let ret: u64;

    asm!("int 0x7f" :
        "={rax}"(ret)
    :
        "{rax}"(vector as u64),
        "{rdi}"(a),
        "{rsi}"(b)
    :: "intel");

    ret
}

unsafe fn syscall3(vector: Syscall, a: u64, b: u64, c: u64) -> u64 {
    let ret: u64;

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

unsafe fn syscall4(vector: Syscall, a: u64, b: u64, c: u64, d: u64) -> u64 {
    let ret: u64;

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
pub unsafe fn alloc_page(base_addr: *mut u8, page_count: u64, flags: u64) -> u64 {
    syscall3(Syscall::AllocPage, base_addr as u64, page_count, flags)
}

#[export_name = "syscall_release_page"]
pub unsafe fn release_page(base_addr: *mut u8, page_count: u64) -> u64 {
    syscall2(Syscall::ReleasePage, base_addr as u64, page_count)
}

#[export_name = "syscall_modify_page"]
pub unsafe fn modify_page(base_addr: *mut u8, page_count: u64, flags: u64) -> u64 {
    syscall3(Syscall::ModifyPage, base_addr as u64, page_count, flags)
}

#[export_name = "syscall_map_physical_memory"]
pub unsafe fn map_physical_memory(base_addr: *mut u8, physical_addr: u64, page_count: u64, flags: u64) -> u64 {
    syscall4(Syscall::MapPhysicalMemory, base_addr as u64, physical_addr, page_count, flags)
}

#[export_name = "syscall_clone_handle"]
pub unsafe fn clone_handle(handle: u64) -> u64  {
    syscall1(Syscall::CloneHandle, handle)
}

#[export_name = "syscall_release_handle"]
pub unsafe fn release_handle(handle: u64) -> u64  {
    syscall1(Syscall::ReleaseHandle, handle)
}

#[export_name = "syscall_create_page_context"]
pub unsafe fn create_page_context() -> u64 {
    syscall0(Syscall::CreatePageContext)
}

#[export_name = "syscall_debug"]
pub unsafe fn debug() -> u64 {
    syscall0(Syscall::Debug)
}

#[export_name = "syscall_set_page_context"]
pub unsafe fn set_page_context(page_ctx: u64) -> u64 {
    syscall1(Syscall::SetPageContext, page_ctx)
}

#[export_name = "syscall_get_page_context"]
pub unsafe fn get_page_context() -> u64 {
    syscall0(Syscall::GetPageContext)
}

#[export_name = "syscall_create_task"]
pub unsafe fn create_task(page_ctx: u64, rip: u64, rsp: u64) -> u64 {
    syscall3(Syscall::CreateTask, page_ctx, rip, rsp)
}

#[export_name = "syscall_exit"]
pub unsafe fn exit(status: u64) -> u64 {
    syscall1(Syscall::Exit, status)
}

#[export_name = "syscall_read_file"]
pub unsafe fn read_file(file: u64, buff: *mut u8, buff_len: u64) -> u64 {
    syscall3(Syscall::ReadFile, file, buff as u64, buff_len)
}

#[export_name = "syscall_write_file"]
pub unsafe fn write_file(file: u64, buff: *const u8, buff_len: u64) -> u64 {
    syscall3(Syscall::WriteFile, file, buff as u64, buff_len)
}

#[export_name = "syscall_open_file"]
pub unsafe fn open_file(path: *const u8, path_len: u64, flags: u64) -> u64 {
    syscall3(Syscall::OpenFile, path as u64, path_len, flags)
}
