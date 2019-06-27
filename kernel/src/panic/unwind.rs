#![cfg_attr(feature = "nightly", feature(unwind_attributes))]
#![cfg_attr(feature = "asm", feature(asm, naked_functions))]

use crate::println;

use core::slice;
use core::fmt::{Debug, Formatter, Result as FmtResult, Write};
use core::ops::{Index, IndexMut};

use gimli::{UnwindSection, UnwindTable, UnwindTableRow, EhFrame, BaseAddresses, UninitializedUnwindContext, Reader, EndianSlice, NativeEndian, CfaRule, RegisterRule, X86, FrameDescriptionEntry, CieOrFde};
use fallible_iterator::FallibleIterator;

pub struct StackFrames<'a> {
    unwinder: &'a mut DwarfUnwinder,
    registers: Registers,
    state: Option<(UnwindTableRow<StaticReader>, u32)>,
}

#[derive(Debug)]
pub struct StackFrame {
    initial_address: u32,
    return_address: u32,
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

#[derive(Debug)]
pub enum UnwindError {
    NoInfoForAddress(u32),
    Gimli(gimli::read::Error),
}

struct UnwindInfo<R: Reader> {
    row: UnwindTableRow<R>,
    initial_address: u32,
}

fn fde_for_address<R: Reader>(eh_frame: &EhFrame<R>, bases: &BaseAddresses, address: u32)
    -> Result<FrameDescriptionEntry<R>, UnwindError>
{
        let mut entry_iter = eh_frame.entries(bases);

        while let Some(entry) = entry_iter.next().map_err(UnwindError::Gimli)? {
            match entry {
                CieOrFde::Cie(_) => {}
                CieOrFde::Fde(partial_fde) => {
                    let fde = partial_fde.parse(EhFrame::cie_from_offset)
                        .map_err(UnwindError::Gimli)?;

                    if fde.contains(address as u64) {
                        return Ok(fde);
                    }
                }
            }
        }

        return Err(UnwindError::NoInfoForAddress(address));
}

impl ObjectRecord {
    fn unwind_info_for_address(
        &self,
        ctx: &mut UninitializedUnwindContext<StaticReader>,
        address: u32,
    ) -> Result<UnwindInfo<StaticReader>, UnwindError> {
        let &ObjectRecord {
            ref eh_frame,
            ref bases,
            ..
        } = self;

        println!(" -- unwind_info_for_address");

        let fde = fde_for_address(eh_frame, bases, address)?;

        println!(" -- FDE = {:x?}", fde);

        let mut result_row = None;
        {
            let mut table = UnwindTable::new(eh_frame, bases, ctx, &fde)
                .map_err(UnwindError::Gimli)?;

            while let Some(row) = table.next_row().map_err(UnwindError::Gimli)? {
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
            None => Err(UnwindError::NoInfoForAddress(address))
        }
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
}

impl<'a> FallibleIterator for StackFrames<'a> {
    type Item = StackFrame;
    type Error = UnwindError;

    fn next(&mut self) -> Result<Option<StackFrame>, Self::Error> {
        let registers = &mut self.registers;

        if let Some((row, cfa)) = self.state.take() {
            let mut newregs = registers.clone();
            newregs[X86::RA] = None;
            for &(reg, ref rule) in row.registers() {
                println!(" -- rule {:x?} {:x?}", reg, rule);
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
            newregs[X86::ESP] = Some(cfa);

            *registers = newregs;
            println!(" -- registers:{:x?}", registers);
        }


        if let Some(return_address) = registers[X86::RA] {
            println!(" -- return address is 0x{:x}", return_address);

            // sub 1 byte from return address to approximate caller address for
            // contains/unwind_info_for_address
            let caller_approx = return_address - 1;

            let rec = if self.unwinder.cfi.er.text.contains(caller_approx) {
                &self.unwinder.cfi
            } else {
                return Err(UnwindError::NoInfoForAddress(caller_approx));
            };

            let UnwindInfo { row, initial_address } = rec.unwind_info_for_address(&mut self.unwinder.ctx, caller_approx)?;

            println!(" -- ok: {:x?} (0x{:x} - 0x{:x})", row.cfa(), row.start_address(), row.end_address());
            let cfa = match *row.cfa() {
                CfaRule::RegisterAndOffset { register, offset } => {
                    match registers[register] {
                        Some(val) => val.wrapping_add(offset as u32),
                        None => {
                            panic!("no value for register: {:x?}", X86::register_name(register));
                        }
                    }
                }
                _ => unimplemented!(),
            };
            println!(" -- cfa is 0x{:x}", cfa);

            self.state = Some((row, cfa));

            Ok(Some(StackFrame {
                initial_address,
                return_address,
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

impl Index<gimli::Register> for Registers {
    type Output = Option<u32>;

    fn index(&self, reg: gimli::Register) -> &Option<u32> {
        &self.registers[reg.0 as usize]
    }
}

impl IndexMut<gimli::Register> for Registers {
    fn index_mut(&mut self, reg: gimli::Register) -> &mut Option<u32> {
        &mut self.registers[reg.0 as usize]
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

#[derive(Debug)]
#[repr(C)]
struct CReg {
    esp: u32,
    ra: u32,
}

fn capture_state(mut f: impl FnMut(&CReg)) {
    use core::mem;
    use core::ffi::c_void;

    extern "C" {
        fn panic_unwind_capture_state(
            data: *mut c_void,
            f: extern fn(data: *mut c_void, reg: *const CReg),
        );
    }

    extern "C" fn bounce(data: *mut c_void, reg: *const CReg) {
        let callback: *mut &mut FnMut(&CReg) = unsafe { mem::transmute(data) };
        let (callback, reg) = unsafe { (&mut *callback, &*reg) };

        callback(reg);
    }

    let mut func_ref: &mut FnMut(&CReg) = &mut f;
    let func_ptr: *mut &mut FnMut(&CReg) = &mut func_ref;
    let ffi_data: *mut c_void = unsafe { mem::transmute(func_ptr) };

    unsafe { panic_unwind_capture_state(ffi_data, bounce) };
}

pub fn trace(w: &mut Write) {
    capture_state(|creg| {
        println!("{:x?}", creg);

        let mut unwinder = DwarfUnwinder::default();

        let mut registers = Registers::default();
        registers[X86::ESP] = Some(creg.esp);
        registers[X86::RA] = Some(creg.ra);

        let mut frames = StackFrames::new(&mut unwinder, registers);

        loop {
            match frames.next() {
                Ok(Some(frame)) => {
                    println!("[{:x?}]", frame.initial_address);
                }
                Ok(None) => {
                    break
                }
                Err(e) => {
                    panic!("StackFrames::next: {:x?}", e);
                }
            }
        }
    });
}
