enum64! {
    enum Syscall {
        1   => AllocPage,
        2   => ReleasePage,
        3   => ModifyPage,
        4   => ReleaseHandle,
        5   => CloneHandle,
        6   => CreatePageContext,
        7   => Debug,
        8   => SetPageContext,
        9   => GetPageContext,
        10  => SpawnTask,
        11  => Exit,
        12  => MapPhysicalMemory,
        13  => ReadFile,
        14  => WriteFile,
    }
}

enum64! {
    enum SysError {
        0xffff_ffff_0000_0001 => BadSyscall,
        0xffff_ffff_0000_0002 => BadPointer,
        0xffff_ffff_0000_0003 => AlreadyMapped,
        0xffff_ffff_0000_0004 => MemoryExhausted,
        0xffff_ffff_0000_0005 => IllegalValue,
        0xffff_ffff_0000_0006 => WrongObjectKind,
        0xffff_ffff_0000_0007 => BadHandle,
        0xffff_ffff_0000_0008 => IoError,
    }
}

pub const OK: u64 = 0;

pub type SysResult<T> = Result<T, SysError>;
