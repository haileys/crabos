use core::cmp;
use x86_64::instructions::port::Port;

use crate::critical;

const PIT_FREQ: usize = 1193182;

unsafe fn set_frequency(hz: usize) {
    let divisor = cmp::min(PIT_FREQ / hz, 65535);

    critical::section(|| {
        let mut port = Port::<u8>::new(0x40);
        port.write(((divisor >> 0) & 0xff) as u8);
        port.write(((divisor >> 8) & 0xff) as u8);
    });
}

pub unsafe fn init() {
    critical::section(|| {
        // initialize pit channel 0
        let mut port = Port::<u8>::new(0x43);
        port.write(0b00110100);

        set_frequency(20);
    });
}
