enum64! {
    enum Syscall {
        1   => AllocPage,
        2   => ReleasePage,
        3   => ModifyPage,
    }
}

enum64! {
    enum SysError {
        0xffff_ffff_0000_0001 => BadSyscall,
        0xffff_ffff_0000_0002 => BadPointer,
        0xffff_ffff_0000_0003 => AlreadyMapped,
        0xffff_ffff_0000_0004 => MemoryExhausted,
        0xffff_ffff_0000_0005 => IllegalValue,
    }
}

pub const OK: u64 = 0;

pub type SysResult<T> = Result<T, SysError>;
