org 0x8000
use16

    %include "kernel/src/consts.asm"

start:
    ; enable A20 line
    mov ax, 0x2401
    int 0x15
    mov si, could_not_enable_a20
    jc error_bios

    ; read memory map
    mov di, memory_map
    xor ebx, ebx
    xor bp, bp
.memory_map_loop:
    mov edx, 0x534d4150
    mov eax, 0xe820
    mov ecx, 24
    int 0x15
    jc .memory_map_done
    test ebx, ebx
    jz .memory_map_done
    add di, 24
    inc bp
    cmp di, memory_map_end
    jb .memory_map_loop
.memory_map_done:
    mov [memory_map_count], bp

    ; load protected mode GDT and a null IDT (we don't need interrupts)
    cli
    lgdt [gdtr32]
    lidt [idtr32]

    ; set protected mode bit of cr0
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; far jump to load CS with 32 bit segment
    jmp 0x08:protected_mode

error_bios: ; pass msg in SI
.loop:
    lodsb
    or al, al
    jz .end
    mov ah, 0x0e
    int 0x10
    jmp .loop
.end:
    cli
    hlt

    use32

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

use32
protected_mode:
    ; load all the other segments with 32 bit data segments
    mov eax, 0x10
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

    ; jump to stage 2 and reload code segment
    jmp stage2

could_not_enable_a20 db "could not enable A20 line", 0
could_not_read_memory_map db "could not read memory map from BIOS", 0

gdtr32:
    dw (gdt32.end - gdt32) - 1 ; size
    dd gdt32                   ; offset

idtr32:
    dw 0
    dd 0

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

; memory map:
memory_map_count dw 0
memory_map      equ 0x4000
memory_map_end  equ 0x4ff8

; must be at end of file
stage2:
