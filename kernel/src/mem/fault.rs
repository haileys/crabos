use crate::interrupt::TrapFrame;

use bitflags::bitflags;

bitflags! {
    pub struct Flags: u64 {
        const PRESENT   = 0x001;
        const WRITE     = 0x002;
        const USER      = 0x004;
    }
}

extern {
    static _bss: u8;
    static _bss_end: u8;
}

pub fn fault(frame: &TrapFrame, flags: Flags, address: *const u8) {
    panic!("Page fault! rip: {:x?}, address: {:?}, flags: {:?}",
        frame.rip,
        address,
        flags);
}
