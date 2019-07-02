ENTRY(start)

SECTIONS {
    . = 0xc0000000;

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

    _end = .;
}
