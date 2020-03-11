use crate::syscall;

pub fn exit(status: u64) -> ! {
    unsafe { syscall::exit(status); }
    unreachable!()
}
