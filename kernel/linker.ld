ENTRY(start)

SECTIONS {
    . = 0xc0000000;

    base = .;

    .text : {
        target/i386-kernel/start.o(.text)
        *(.text)
    }

    .rodata : {
        *(.rodata)
    }

    roend = ALIGN(0x1000);

    .data ALIGN(0x1000) : {
        *(.data)
    }

    bssbase = ALIGN(0x1000);

    .bss ALIGN(0x1000) : {
        *(.bss)
    }

    .stack ALIGN(0x100) : {
        *(.stack)
    }

    end = ALIGN(0x1000);

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
    }
}
