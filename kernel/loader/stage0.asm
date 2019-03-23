org 0x7c00
use16

    %include "kernel/src/consts.asm"
    KERNEL_SECTORS equ (KERNEL_SIZE + 511) / 512

    ; initialize cs
    jmp 0:start
start:
    ; initialize data segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; initialize stack
    mov sp, 0x7c00

    ; print starting message
    mov si, boot_msg
    call print

    ; load main kernel code
    mov cx, KERNEL_SECTORS
.read_loop:
    ; perform LBA read, dl is set to boot drive by BIOS
    mov si, pkt
    mov ah, 0x42
    int 0x13

    ; check error
    jc read_error

    ; update LBA packet for next sector:
    add word [pkt.bufseg], 512 >> 4
    inc dword [pkt.lbalo]

    ; loop
    loop .read_loop

    ; jump to kernel start
    jmp KERNEL_PHYS_BASE

align 4
pkt:
    .size   db 16
    .res    db 0
    .count  dw 1
    .bufoff dw 0
    .bufseg dw KERNEL_PHYS_BASE >> 4
    .lbalo  dd 1
    .lbahi  dd 0

boot_msg db "Starting... ", 0

read_error:
    mov si, .msg
    call print
    cli
    hlt

    .msg db "Could not read startup disk", 0

print:
    lodsb
    or al, al
    jz .end
    mov ah, 0x0e
    int 0x10
    jmp print
.end:
    ret

times 510-($-$$) db 0
db 0x55
db 0xaa

