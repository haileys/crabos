use core::fmt::{self, Write};
use core::ptr;

use x86_64::instructions::port::Port;

use crate::console::{self, Console};
use crate::mem::page::{self, PageFlags, PAGE_SIZE};
use crate::mem::phys::{Phys, RawPhys};

const VRAM_SIZE: usize = 8 * 1024 * 1024;
const TEXT_ATTR: u8 = 0x07; // light grey on black

#[repr(align(4096))]
struct Vram {
    buff: [u8; VRAM_SIZE],
}

type BiosFont = [u8; 4096];
static mut BIOS_FONT: BiosFont = [0u8; 4096];

#[link_section=".unalloc"]
static mut VRAM: Vram = Vram { buff: [0; VRAM_SIZE] };

#[repr(packed)]
pub struct VbeModeInfo {
    attrs: u16,
    win_a: u8,
    win_b: u8,
    granularity: u16,
    winsize: u16,
    seg_a: u16,
    seg_b: u16,
    real_fct_ptr: u32,
    pitch: u16, // bytes per scanline
    x_res: u16,
    y_res: u16,
    w_char: u8,
    y_char: u8,
    planes: u8,
    bpp: u8,
    banks: u8,
    memory_model: u8,
    bank_size: u8,
    image_pages: u8,
    reserved0: u8,
    red_mask: u8,
    red_pos: u8,
    green_mask: u8,
    green_pos: u8,
    blue_mask: u8,
    blue_pos: u8,
    rsv_mask: u8,
    rsv_pos: u8,
    directcolor_attrs: u8,
    phys_base: u32,
    reserved1: u32,
    reserved2: u32,
}

#[no_mangle]
pub unsafe extern "C" fn console_init(vbe_mode_info: *const VbeModeInfo, bios_font: *const u8) {
    // copy bios font from low memory
    unsafe { ptr::copy(bios_font, &mut BIOS_FONT as *mut BiosFont as *mut u8, 4096); }

    let vbe_mode_info = unsafe { &*vbe_mode_info };

    let virt = &mut VRAM as *mut Vram as *mut u8;

    for off in (0..VRAM_SIZE).step_by(PAGE_SIZE) {
        let phys = Phys::new(RawPhys(vbe_mode_info.phys_base as u64 + off as u64));
        let virt = virt.add(off);

        page::map(phys, virt, PageFlags::PRESENT | PageFlags::WRITE)
            .expect("page::map in console_init");
    }

    let mut vga = VgaText {
        vram: virt,
        bios_font: unsafe { &BIOS_FONT },
        width: vbe_mode_info.x_res as usize,
        height: vbe_mode_info.y_res as usize,
        pitch: vbe_mode_info.pitch as usize,
        col: 0,
        row: 0,
    };

    vga.blank();

    console::set(Console::VgaText(vga));
}

pub struct VgaText {
    vram: *mut u8,
    bios_font: &'static BiosFont,
    width: usize,
    height: usize,
    pitch: usize,
    col: usize,
    row: usize,
}

const RED: u8 = 0xed;
const GREEN: u8 = 0xbd;
const BLUE: u8 = 0xa6;

const CHAR_HEIGHT: usize = 16;
const CHAR_WIDTH: usize = 8;
const STRIDE: usize = 3;

impl VgaText {
    fn blank(&mut self) {
        unsafe {
            for y in 0..self.height {
                let line = self.vram.add(y * self.pitch);

                for x in 0..self.width {
                    ptr::write(line.add(x * 3 + 0), BLUE);
                    ptr::write(line.add(x * 3 + 1), GREEN);
                    ptr::write(line.add(x * 3 + 2), RED);
                }
            }
        }

        // draw the crab
        let crab = include_bytes!("../../../crab.bmp").as_ptr();
        let crab_header = 0x36;
        let crab_width = 256;
        let crab_height = 256;

        unsafe {
            for y in 0..crab_height {
                let line = self.vram
                    .add((self.height - crab_height + y) * self.pitch)
                    .add((self.width - crab_width) * STRIDE);
                let crab_idx = y * crab_width * STRIDE + crab_header;
                ptr::copy(unsafe { crab.add(crab_idx) }, line, crab_width * STRIDE);
            }
        }
    }

    fn rows(&self) -> usize {
        self.height / CHAR_HEIGHT
    }

    fn cols(&self) -> usize {
        (self.width - 256) / CHAR_WIDTH
    }

    fn newline(&mut self) {
        self.row += 1;
        self.col = 0;

        if self.row == self.rows() {
            let line0 = self.vram;
            let line1 = unsafe { self.vram.add(self.pitch * CHAR_HEIGHT) };

            let count = self.pitch * (self.rows() - 1) * CHAR_HEIGHT;
            unsafe { ptr::copy(line1, line0, count); }

            // blank last line
            let last_line = unsafe { self.vram.add(count) };

            unsafe {
                for y in 0..CHAR_HEIGHT {
                    let line = last_line.add(self.pitch * y);

                    for x in 0..self.width {
                        ptr::write(line.add(x * 3 + 0), BLUE);
                        ptr::write(line.add(x * 3 + 1), GREEN);
                        ptr::write(line.add(x * 3 + 2), RED);
                    }
                }
            }

            self.row -= 1;
        }
    }

    fn write_cp437(&mut self, b: u8) {
        let pos = self.row * CHAR_HEIGHT * self.pitch
                + self.col * CHAR_WIDTH * STRIDE;

        // write directly to VRAM:
        unsafe {
            for glyph_y in 0..CHAR_HEIGHT {
                let glyph = self.bios_font[b as usize * 16 + glyph_y];

                let pos = pos + glyph_y * self.pitch;

                for glyph_x in 0..CHAR_WIDTH {
                    let pos = pos + glyph_x * STRIDE;

                    if (glyph & (0x80 >> glyph_x)) != 0 {
                        unsafe {
                            ptr::write_volatile(self.vram.add(pos + 0), 0);
                            ptr::write_volatile(self.vram.add(pos + 1), 0);
                            ptr::write_volatile(self.vram.add(pos + 2), 0);
                        }
                    }
                }
            }
        }

        self.col += 1;

        if self.col == self.cols() {
            self.newline();
        }
    }

    fn update_cursor(&self) {
        // let mut reg = Port::<u8>::new(0x3d4);
        // let mut val = Port::<u8>::new(0x3d5);

        // let pos = self.row * self.width + self.col;

        // unsafe {
        //     reg.write(0x0f);
        //     val.write((pos & 0xff) as u8);

        //     reg.write(0x0e);
        //     val.write(((pos >> 8) & 0xff) as u8);
        // }
    }
}

impl Write for VgaText {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match c {
                '\n' => self.newline(),
                c if c.is_ascii() => self.write_cp437(c as u8),
                _ => self.write_cp437(b'?'),
            }
        }

        self.update_cursor();

        Ok(())
    }
}
