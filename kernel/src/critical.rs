use core::ops::Drop;

pub struct Critical(bool);

const FLAG_IF: u32 = 1 << 9;

fn eflags() -> u32 {
    unsafe {
        let eflags: u32;
        asm!("pushf; pop $0" : "=r"(eflags));
        eflags
    }
}

pub fn begin() -> Critical {
    let if_ = (eflags() & FLAG_IF) != 0;
    unsafe { asm!("cli" :::: "volatile"); }
    Critical(if_)
}

pub fn section<T>(f: impl FnOnce() -> T) -> T {
    let _crit = begin();
    f()
}

impl Drop for Critical {
    fn drop(&mut self) {
        if self.0 {
            unsafe { asm!("sti" :::: "volatile"); }
        }
    }
}
