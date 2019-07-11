ENTRY(start)

SECTIONS {
    . = 0xffff800000000000;

    _base = .;

    _text = .;

    .text : ALIGN(0x1000) {
        target/x86_64-kernel/start.o(.text)
        *(.text)
        *(.text.*)
    }

    _text_end = .;

    _rodata = .;

    .rodata : ALIGN(0x1000) {
        *(.rodata)
        *(.rodata.*)
        *(.got)
        *(.got.*)
    }
    _rodata_end = .;

    _data = .;

    .data : ALIGN(0x1000) {
        *(.data)
        *(.data.*)
    }

    _data_end = .;

    _bss = .;

    .bss : ALIGN(0x1000) {
        *(.bss)
        *(.bss.*)
    }

    .stack : ALIGN(0x1000) {
        *(.stack)
        *(.stack.*)
    }

    _bss_end = .;

    _phys_rc = .;

    /* 48 physical address lines,
       divided by page size,
       multiplied by 8 (size of u64) */

    . += (1 << 48) / 4096 * 8;

    _phys_rc_end = .;

    _end = .;

    /DISCARD/ : {
        *(.debug_abbrev)
        *(.debug_frame)
        *(.debug_info)
        *(.debug_line)
        *(.debug_loc)
        *(.debug_macinfo)
        *(.debug_pubnames)
        *(.debug_pubtypes)
        *(.debug_ranges)
        *(.debug_str)
        *(.note.GNU-stack)

        /* why does nasm only generate these sections for start.o? */
        target/x86_64-kernel/start.o(.igot.plt)
        target/x86_64-kernel/start.o(.iplt)
        target/x86_64-kernel/start.o(.rela.got)
        target/x86_64-kernel/start.o(.rela.iplt)
        target/x86_64-kernel/start.o(.rela.text)
    }
}
