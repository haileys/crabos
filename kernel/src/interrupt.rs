use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;

pub const IRQ_BASE: u8 = 0x20;

macro_rules! interrupts {
    ($($vector:expr => $name:ident,)*) => {
        #[derive(Debug)]
        pub enum Interrupt {
            $($name,)*
            Irq(u8),
            Other(u8),
        }

        impl From<u8> for Interrupt {
            fn from(vector: u8) -> Self {
                match vector {
                    $($vector => Interrupt::$name,)*
                    _ => {
                        if vector >= IRQ_BASE && vector < IRQ_BASE + 0x10 {
                            Interrupt::Irq(vector - IRQ_BASE)
                        } else {
                            Interrupt::Other(vector)
                        }
                    }
                }
            }
        }

        impl Into<u8> for Interrupt {
            fn into(self) -> u8 {
                match self {
                    $(Interrupt::$name => $vector,)*
                    Interrupt::Irq(irq) => irq + IRQ_BASE,
                    Interrupt::Other(vector) => vector,
                }
            }
        }
    }
}

interrupts! {
    0x00 => DivideByZero,
    0x01 => Debug,
    0x02 => Nmi,
    0x03 => Breakpoint,
    0x04 => Overflow,
    0x05 => BoundRangeExceeded,
    0x06 => InvalidOpcode,
    0x07 => DeviceNotAvailable,
    0x08 => DoubleFault,
    0x0a => InvalidTss,
    0x0b => SegmentNotPresent,
    0x0c => StackSegmentFault,
    0x0d => GeneralProtectionFault,
    0x0e => PageFault,
    0x10 => X87Exception,
    0x11 => AlignmentCheck,
    0x12 => MachineCheck,
    0x13 => SimdException,
    0x14 => VirtualizationException,
    0x1e => SecurityException,
}

#[repr(C)]
#[derive(Debug)]
pub struct Registers {
    // general purpose registers, see isrs.asm
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9:  u64,
    r8:  u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
    rbx: u64,
    rdx: u64,
    rcx: u64,
    rax: u64,
    // segment registers
    // es: u32,
    // ds: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
    regs: Registers,

    // interrupt details
    interrupt_vector: u64,
    error_code: u64,

    // interrupt stack frame
    eip: u64,
    cs: u64,
    eflags: u64,
    esp: u64,
    ss: u64,
}

impl TrapFrame {
    pub fn interrupt(&self) -> Interrupt {
        (self.interrupt_vector as u8).into()
    }
}

#[no_mangle]
pub extern "C" fn interrupt(frame: &TrapFrame) {
    // crate::println!("{:#x?}", frame);

    match frame.interrupt() {
        Interrupt::Irq(irq) => {
            let mut pic1 = Port::<u8>::new(0x20);
            let mut pic2 = Port::<u8>::new(0xa0);

            if irq == 0 {
                // PIT
                crate::print!(".")
            }

            if irq == 1 {
                // keyboard
                let mut keyboard = Port::<u8>::new(0x60);
                unsafe { keyboard.read(); }
            }

            // acknowledge interupt:
            unsafe { pic1.write(0x20); }

            if irq >= 0x08 {
                // irq from pic 2, send separate ack
                unsafe { pic2.write(0x20); }
            }
        }
        Interrupt::PageFault => {
            use crate::mem::fault::{fault, Flags};

            let flags = Flags::from_bits(frame.error_code)
                .expect("mem::fault::Flags::from_bits");

            let address = Cr2::read().as_ptr();

            fault(frame, flags, address);
        }
        Interrupt::Other(vector) => {
            panic!("unexpected interrupt: {:#2x}", vector);
        }
        exception => {
            panic!("CPU exception: {:?}", exception);
        }
    }
}
