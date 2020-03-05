ENTRY(_start)

SECTIONS {
    . = 0x10000000;

    _base = .;

    .text : ALIGN(0x1000) {
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
        *(.rela.*)
        *(.iplt)
        *(.igot.*)
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

    . = ALIGN(0x1000);
    _bss_end = .;

    /DISCARD/ : {
        *(.debug_abbrev)
        *(.debug_aranges)
        *(.debug_frame)
        *(.debug_gdb_scripts)
        *(.debug_info)
        *(.debug_line)
        *(.debug_loc)
        *(.debug_macinfo)
        *(.debug_pubnames)
        *(.debug_pubtypes)
        *(.debug_ranges)
        *(.debug_str)
        *(.note.GNU-stack)
    }
}
