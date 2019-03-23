use32

%include "kernel/src/consts.asm"

extern base
extern bssbase
extern end
extern main

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

    ; identity map first (currently executing) page of kernel in pt 0
    mov dword [EARLY_PHYS(early_pt_0) + ((KERNEL_PHYS_BASE >> 12) * 4)], KERNEL_PHYS_BASE | PAGE_PRESENT | PAGE_WRITABLE

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
    or eax, PAGE_PRESENT | PAGE_WRITABLE
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

    jmp main

section .bss
    align PAGE_SIZE
    early_pd    resb PAGE_SIZE
    early_pt_0  resb PAGE_SIZE
    early_pt_k  resb PAGE_SIZE

section .stack
    global stackguard
    align PAGE_SIZE
    stackguard times PAGE_SIZE db 0
    global stack
    stack times PAGE_SIZE db 0
    global stackend
    stackend equ stack + PAGE_SIZE
