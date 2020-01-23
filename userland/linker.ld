OUTPUT_FORMAT(binary)
ENTRY(_start)

SECTIONS {
    . = 0x100000000;

    .text : ALIGN(0x1000) {
        *(.text)
        *(.text.*)
    }

    .rodata : ALIGN(0x1000) {
        *(.rodata)
        *(.rodata.*)
        *(.got)
        *(.got.*)
        *(.rela.*)
        *(.iplt)
        *(.igot.*)
    }
}
