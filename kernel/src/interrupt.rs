use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;
use x86_64::registers::rflags::RFlags;

use crate::task::{self, SEG_UCODE, SEG_UDATA};
use crate::syscall;

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
    0x7f => Syscall,
}

#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct Registers {
    // general purpose registers, see isrs.asm
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9:  u64,
    pub r8:  u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,
    // segment registers
    // es: u32,
    // ds: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TrapFrame {
    pub regs: Registers,

    // interrupt details
    pub interrupt_vector: u64,
    pub error_code: u64,

    // interrupt stack frame
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl TrapFrame {
    pub fn new(rip: u64, rsp: u64) -> Self {
        TrapFrame {
            regs: Default::default(),
            interrupt_vector: 0,
            error_code: 0,
            rip: rip,
            cs: SEG_UCODE as u64,
            rflags: RFlags::INTERRUPT_FLAG.bits(),
            rsp: rsp,
            ss: SEG_UDATA as u64,
        }
    }
}

impl TrapFrame {
    pub fn interrupt(&self) -> Interrupt {
        (self.interrupt_vector as u8).into()
    }
}

#[no_mangle]
pub extern "C" fn interrupt(frame: &mut TrapFrame) {
    // crate::println!("{:#x?}", frame);

    match frame.interrupt() {
        Interrupt::Irq(irq) => {
            let mut pic1 = Port::<u8>::new(0x20);
            let mut pic2 = Port::<u8>::new(0xa0);

            if irq == 0 {
                // PIT
                unsafe { task::switch(frame); }
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
        Interrupt::Syscall => {
            unsafe { task::dispatch_syscall(frame); }
        }
        Interrupt::Other(vector) => {
            panic!("unexpected interrupt: {:#2x}", vector);
        }
        exception => {
            panic!("CPU exception: {:?}", exception);
        }
    }
}
