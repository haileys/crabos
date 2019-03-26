use32

%include "kernel/src/consts.asm"

extern base
extern bssbase
extern roend
extern end
extern main
extern phys_init_regions

global start

; 16 bit start code:
start:
    incbin "target/loader/stage1.bin"

; the kernel is linked with base = 0xc0000000, but the early bootloader
; places us at 0x8000 without paging enabled. we need to be careful in this
; early phase to translate symbol addresses to physical addresses:
%define EARLY_PHYS(addr) ((addr) - KERNEL_BASE + KERNEL_PHYS_BASE)

use32
protected_mode:
    ; zero bss pages
    xor eax, eax
    mov edi, EARLY_PHYS(bssbase)
    mov ecx, end
    sub ecx, bssbase
    shr ecx, 2 ; div 4
    rep stosd

    ; set up recursive pd map
    mov eax, EARLY_PHYS(early_pd)
    or eax, PAGE_PRESENT | PAGE_WRITABLE
    mov [EARLY_PHYS(early_pd) + 1023 * 4], eax

    ; set up pd entry for pt 0
    mov eax, EARLY_PHYS(early_pt_0)
    or eax, PAGE_PRESENT | PAGE_WRITABLE
    mov dword [EARLY_PHYS(early_pd)], eax

    ; identity map first MiB of phys memory, except null page
    mov esi, 0x1000
.map_low:
    mov eax, esi
    or eax, PAGE_PRESENT | PAGE_WRITABLE
    mov ebx, esi
    shr ebx, 12
    mov dword [EARLY_PHYS(early_pt_0) + ebx * 4], eax
    add esi, PAGE_SIZE
    cmp esi, 0x100000
    jb .map_low

    ; setup pd entry for pt k
    mov eax, EARLY_PHYS(early_pt_k)
    or eax, PAGE_PRESENT | PAGE_WRITABLE
    mov dword [EARLY_PHYS(early_pd) + ((0xc0000000 >> 22) * 4)], eax

    ; map kernel in pt k
    mov edi, EARLY_PHYS(early_pt_k)
    mov esi, base
.map_kernel:
    ; build pt k entry
    mov eax, esi
    sub eax, KERNEL_BASE - KERNEL_PHYS_BASE
    or eax, PAGE_PRESENT
    cmp esi, roend
    jb .no_write
    or eax, PAGE_WRITABLE
.no_write:
    stosd
    ; compare to end
    add esi, PAGE_SIZE
    cmp esi, end
    jb .map_kernel

    ; set cr3
    mov eax, EARLY_PHYS(early_pd)
    mov cr3, eax

    ; enable paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ; jump to kernel in higher half
    mov eax, higher_half
    jmp eax

higher_half:
    ; unmap stack guard
    mov ebx, stackguard
    shr ebx, 12
    mov [0xffc00000 + ebx * 4], dword 0

    ; set up kernel stack
    mov esp, stackend
    xor ebp, ebp

    ; init phys allocator
    mov eax, [EARLY_MEMORY_MAP_LEN]
    push eax
    push EARLY_MEMORY_MAP
    call phys_init_regions
    add esp, 8

    ; unmap low memory
    xor eax, eax
    mov edi, PAGE_TABLES
    mov ecx, 1024
    rep stosd

    ; reload GDT in high memory
    lgdt [gdtr]

    ; flush TLB
    mov eax, cr3
    mov cr3, eax

    jmp main

section .data
gdtr:
    dw (gdt.end - gdt) - 1 ; size
    dd gdt                 ; offset

gdt:
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

section .bss
    align PAGE_SIZE
    early_pd    resb PAGE_SIZE
    early_pt_0  resb PAGE_SIZE
    early_pt_k  resb PAGE_SIZE
    memory_map  resb PAGE_SIZE
    global temp_page
    temp_page   resb PAGE_SIZE

section .stack
    global stackguard
    align PAGE_SIZE
    stackguard times PAGE_SIZE db 0
    global stack
    stack times PAGE_SIZE db 0
    global stackend
    stackend equ stack + PAGE_SIZE
