#![cfg_attr(feature = "nightly", feature(unwind_attributes))]
#![cfg_attr(feature = "asm", feature(asm, naked_functions))]

use crate::println;

use core::slice;
use core::fmt::{Debug, Formatter, Result as FmtResult, Write, self};
use core::ops::{Index, IndexMut};

use gimli::{UnwindSection, UnwindTable, UnwindTableRow, EhFrame, BaseAddresses, UninitializedUnwindContext, Pointer, Reader, EndianSlice, NativeEndian, CfaRule, RegisterRule, EhFrameHdr, ParsedEhFrameHdr, X86, FrameDescriptionEntry, CieOrFde};
use fallible_iterator::FallibleIterator;

pub struct StackFrames<'a> {
    unwinder: &'a mut DwarfUnwinder,
    registers: Registers,
    state: Option<(UnwindTableRow<StaticReader>, u32)>,
}

#[derive(Debug)]
pub struct StackFrame {
    initial_address: u32,
}

impl StackFrame {
    pub fn initial_address(&self) -> u32 {
        self.initial_address
    }
}

pub trait Unwinder: Default {
    fn trace<F>(&mut self, f: F) where F: FnMut(&mut StackFrames);
}

type StaticReader = EndianSlice<'static, NativeEndian>;

struct ObjectRecord {
    er: EhRef,
    eh_frame: EhFrame<StaticReader>,
    bases: BaseAddresses,
}

pub struct DwarfUnwinder {
    cfi: ObjectRecord,
    ctx: UninitializedUnwindContext<StaticReader>,
}

unsafe fn section<'a>(start: &'a u8, end: &'a u8) -> &'a [u8] {
    let len = (end as *const u8).offset_from(start as *const u8) as usize;
    slice::from_raw_parts(start, len)
}

impl Default for DwarfUnwinder {
    fn default() -> DwarfUnwinder {
        let er = find_cfi_section();
        let cfi =
            unsafe {
                // TODO: set_got()
                let bases = BaseAddresses::default()
                    // .set_eh_frame_hdr(er.eh_frame_hdr.start as u64)
                    .set_text(er.text.start as u64);

                let eh_frame_data: &'static [u8] = section(&_eh_frame, &_eh_frame_end);
                println!("eh_frame at {:p} len {:x}", eh_frame_data.as_ptr(), eh_frame_data.len());
                let eh_frame = EhFrame::new(eh_frame_data, NativeEndian);

                let bases = bases.set_eh_frame(eh_frame_data.as_ptr() as u64);

                ObjectRecord { er, eh_frame, bases }
            };

        DwarfUnwinder {
            cfi,
            ctx: UninitializedUnwindContext::new(),
        }
    }
}

impl Unwinder for DwarfUnwinder {
    fn trace<F>(&mut self, mut f: F) where F: FnMut(&mut StackFrames) {
        let registers = Registers::get();

        let mut frames = StackFrames::new(self, registers);

        f(&mut frames)
    }
}

struct UnwindInfo<R: Reader> {
    row: UnwindTableRow<R>,
    initial_address: u32,
}

fn fde_for_address<R: Reader>(eh_frame: &EhFrame<R>, bases: &BaseAddresses, address: u32)
    -> gimli::read::Result<FrameDescriptionEntry<R>>
{
        let mut entry_iter = eh_frame.entries(bases);

        while let Some(entry) = entry_iter.next()? {
            match entry {
                CieOrFde::Cie(_) => {}
                CieOrFde::Fde(partial_fde) => {
                    let fde = partial_fde.parse(EhFrame::cie_from_offset)?;

                    if fde.contains(address as u64) {
                        return Ok(fde);
                    }
                }
            }
        }

        return Err(gimli::read::Error::NoUnwindInfoForAddress);
}

impl ObjectRecord {
    fn unwind_info_for_address(
        &self,
        ctx: &mut UninitializedUnwindContext<StaticReader>,
        address: u32,
    ) -> gimli::read::Result<UnwindInfo<StaticReader>> {
        let &ObjectRecord {
            ref eh_frame,
            ref bases,
            ..
        } = self;

        println!(" -- unwind_info_for_address");

        let fde = fde_for_address(eh_frame, bases, address)?;

        println!(" -- FDE = {:?}", fde);

        let mut result_row = None;
        {
            let mut table = UnwindTable::new(eh_frame, bases, ctx, &fde)?;
            while let Some(row) = table.next_row()? {
                if row.contains(address as u64) {
                    result_row = Some(row.clone());
                    break;
                }
            }
        }

        match result_row {
            Some(row) => Ok(UnwindInfo {
                row,
                initial_address: fde.initial_address() as u32,
            }),
            None => Err(gimli::Error::NoUnwindInfoForAddress)
        }
    }
}

unsafe fn deref_ptr(ptr: Pointer) -> u32 {
    match ptr {
        Pointer::Direct(x) => x as u32,
        Pointer::Indirect(x) => *(x as *const u32),
    }
}


impl<'a> StackFrames<'a> {
    pub fn new(unwinder: &'a mut DwarfUnwinder, registers: Registers) -> Self {
        StackFrames {
            unwinder,
            registers,
            state: None,
        }
    }

    pub fn registers(&mut self) -> &mut Registers {
        &mut self.registers
    }
}

impl<'a> FallibleIterator for StackFrames<'a> {
    type Item = StackFrame;
    type Error = gimli::Error;

    fn next(&mut self) -> Result<Option<StackFrame>, Self::Error> {
        let registers = &mut self.registers;

        if let Some((row, cfa)) = self.state.take() {
            let mut newregs = registers.clone();
            newregs[X86::RA] = None;
            for &(reg, ref rule) in row.registers() {
                println!(" -- rule {:?} {:?}", reg, rule);
                assert!(reg != X86::ESP); // stack = cfa
                newregs[reg] = match *rule {
                    RegisterRule::Undefined => unreachable!(), // registers[reg],
                    RegisterRule::SameValue => Some(registers[reg].expect("registers[reg]")), // not sure why this exists
                    RegisterRule::Register(r) => registers[r],
                    RegisterRule::Offset(n) => Some(unsafe { *((cfa.wrapping_add(n as u32)) as *const u32) }),
                    RegisterRule::ValOffset(n) => Some(cfa.wrapping_add(n as u32)),
                    RegisterRule::Expression(_) => unimplemented!(),
                    RegisterRule::ValExpression(_) => unimplemented!(),
                    RegisterRule::Architectural => unreachable!(),
                };
            }
            newregs[7] = Some(cfa);

            *registers = newregs;
            println!(" -- registers:{:?}", registers);
        }


        if let Some(mut caller) = registers[X86::RA] {
            caller -= 1; // THIS IS NECESSARY
            println!(" -- caller is 0x{:x}", caller);

            let rec = if self.unwinder.cfi.er.text.contains(caller) {
                &self.unwinder.cfi
            } else {
                return Err(gimli::Error::NoUnwindInfoForAddress);
            };

            let UnwindInfo { row, initial_address } = rec.unwind_info_for_address(&mut self.unwinder.ctx, caller)?;

            println!(" -- ok: {:?} (0x{:x} - 0x{:x})", row.cfa(), row.start_address(), row.end_address());
            let cfa = match *row.cfa() {
                CfaRule::RegisterAndOffset { register, offset } => {
                    match registers[register] {
                        Some(val) => val.wrapping_add(offset as u32),
                        None => {
                            panic!("no value for register: {:?}", X86::register_name(register));
                        }
                    }
                }
                _ => unimplemented!(),
            };
            println!(" -- cfa is 0x{:x}", cfa);

            self.state = Some((row, cfa));

            Ok(Some(StackFrame {
                initial_address,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Registers {
    registers: [Option<u32>; 9],
}

extern "C" {
    fn panic_unwind_get_return_addr() -> u32;
}

impl Registers {
    pub fn get() -> Registers {
        let mut regs = Registers {
            registers: Default::default(),
        };

        let esp;

        let ra = unsafe { panic_unwind_get_return_addr() };
        unsafe { asm!("movl %esp, $0" : "=r"(esp)) };

        regs[X86::RA] = Some(ra);
        regs[X86::ESP] = Some(esp);

        regs
    }
}

impl Debug for Registers {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        for reg in &self.registers {
            match *reg {
                None => write!(fmt, " XXX")?,
                Some(x) => write!(fmt, " 0x{:x}", x)?,
            }
        }
        Ok(())
    }
}

impl Index<u16> for Registers {
    type Output = Option<u32>;

    fn index(&self, index: u16) -> &Option<u32> {
        &self.registers[index as usize]
    }
}

impl IndexMut<u16> for Registers {
    fn index_mut(&mut self, index: u16) -> &mut Option<u32> {
        &mut self.registers[index as usize]
    }
}

impl Index<gimli::Register> for Registers {
    type Output = Option<u32>;

    fn index(&self, reg: gimli::Register) -> &Option<u32> {
        &self[reg.0]
    }
}

impl IndexMut<gimli::Register> for Registers {
    fn index_mut(&mut self, reg: gimli::Register) -> &mut Option<u32> {
        &mut self[reg.0]
    }
}

#[derive(Debug)]
pub struct EhRef {
    pub text: AddrRange,
    pub eh_frame_end: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddrRange {
    pub start: u32,
    pub end: u32,
}

impl AddrRange {
    pub fn contains(&self, addr: u32) -> bool {
        addr >= self.start && addr < self.end
    }

    pub fn len(&self) -> u32 {
        self.end - self.start
    }
}

extern "C" {
    static _base: u8;
    static _text_end: u8;
    static _eh_frame: u8;
    static _eh_frame_end: u8;
}

pub fn find_cfi_section() -> EhRef {
    let cfi = unsafe {
        // Safety: None of those are actual accesses - we only get the address
        // of those values.
        let text = AddrRange {
            start: &_base as *const _ as u32,
            end: &_text_end as *const _ as u32,
        };
        let eh_frame_end = &_eh_frame_end as *const _ as u32;

        EhRef {
            text,
            eh_frame_end,
        }
    };
    println!(" -- CFI section: {:x?}", cfi);
    cfi
}

pub fn trace(w: &mut Write) -> Result<(), fmt::Error> {
    use fallible_iterator::FallibleIterator;

    DwarfUnwinder::default().trace(|x| {
        while let Some(frame) = x.next().expect("StackFrames::next") {
            println!("[{:x?}]", frame.initial_address);
        }
    });

    Ok(())
}
