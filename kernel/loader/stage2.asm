%include "kernel/src/consts.asm"

org KERNEL_PHYS_BASE
use32

    ; load all the other segments with 32 bit data segments
    mov eax, SEG_KDATA
    mov ds, eax
    mov es, eax
    mov fs, eax
    mov gs, eax
    mov ss, eax

    ; clear screen
    mov ax, 0x0f
    mov edi, 0xb8000
    mov ecx, 80*25
    rep stosw

    ; check if extended processor information is supported by cpuid
    mov eax, 0x80000000
    cpuid
    mov esi, no_extended_processor_information
    cmp eax, 0x80000001
    jb error

    ; check if long mode is supported
    mov eax, 0x80000001
    cpuid
    mov esi, no_long_mode
    test edx, 1 << 29
    jz error

    ; jump to stage 3 and reload code segment
    jmp stage3

error: ; pass msg in ESI
    mov edi, 0xb8000
    mov ah, 0x4f ; white on red
.loop:
    lodsb
    or al, al
    jz .end
    stosw
    jmp .loop
.end:
    cli
    hlt

no_extended_processor_information db "No extended processor information - 64 bit mode not supported on this CPU", 0
no_long_mode db "No long mode - 64 bit mode not supported on this CPU", 0

gdtr32:
    dw (gdt32.end - gdt32) - 1 ; size
    dd gdt32                   ; offset

gdt32:
    ; null entry
    dq 0
    ; code entry
    dw 0xffff       ; limit 0:15
    dw 0x0000       ; base 0:15
    db 0x00         ; base 16:23
    db 0b10011010   ; access byte - code
    db 0xcf         ; flags/(limit 16:19). 4 KB granularity + 32 bit mode flags
    db 0x00         ; base 24:31
    ; data entry
    dw 0xffff       ; limit 0:15
    dw 0x0000       ; base 0:15
    db 0x00         ; base 16:23
    db 0b10010010   ; access byte - data
    db 0xcf         ; flags/(limit 16:19). 4 KB granularity + 32 bit mode flags
    db 0x00         ; base 24:31
.end:

; must be at end of file
stage3:
