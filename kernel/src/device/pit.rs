use core::cmp;
use x86::io;

use crate::critical;

const PIT_FREQ: usize = 1193182;

unsafe fn set_frequency(hz: usize) {
    let divisor = cmp::min(PIT_FREQ / hz, 65535);

    critical::section(|| {
        io::outb(0x40, ((divisor >> 0) & 0xff) as u8);
        io::outb(0x40, ((divisor >> 8) & 0xff) as u8);
    });
}

pub unsafe fn init() {
    critical::section(|| {
        // initialize pit channel 0
        io::outb(0x43, 0b00110100);

        set_frequency(20);
    });
}
