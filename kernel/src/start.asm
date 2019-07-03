use32

%include "kernel/src/consts.asm"

extern _base
extern _bss
extern _rodata_end
extern _end
extern main
extern phys_init_regions
extern isrs_init

global start

; 16 bit start code:
start:
    incbin "target/loader/stage1.bin"

; the kernel is linked with base = 0xffff800000000000, but the early bootloader
; places us at 0x8000 without paging enabled. we need to be careful in this
; early phase to translate symbol addresses to physical addresses:
%define EARLY_PHYS(addr) ((addr) - KERNEL_BASE + KERNEL_PHYS_BASE)

bits 32
protected_mode:
    ; zero bss pages
    xor eax, eax
    mov edi, EARLY_PHYS(_bss)
    mov ecx, EARLY_PHYS(_end)
    sub ecx, EARLY_PHYS(_bss)
    shr ecx, 2 ; div 4
    rep stosd

    ; set up recursive pml4 map
    mov eax, EARLY_PHYS(pml4)
    or eax, PAGE_PRESENT | PAGE_WRITABLE
    mov [EARLY_PHYS(pml4) + 511 * 8], eax

    ; set up pdpt for early identity map
    mov eax, EARLY_PHYS(pdpt_0)
    or eax, PAGE_PRESENT | PAGE_WRITABLE
    mov dword [EARLY_PHYS(pml4)], eax

    ; set up pd for early identity map
    mov eax, EARLY_PHYS(pd_0)
    or eax, PAGE_PRESENT | PAGE_WRITABLE
    mov [EARLY_PHYS(pdpt_0)], eax

    ; identity map first 4 MiB of phys memory
    mov eax, PAGE_PRESENT | PAGE_WRITABLE | PAGE_HUGE
    mov [EARLY_PHYS(pd_0)], eax

    ; load pml4 into cr3
    mov eax, EARLY_PHYS(pml4)
    mov cr3, eax

    ; enable various extensions in cr4
    %define CR4_PAGE_SIZE_EXT (1 << 4)
    %define CR4_PHYS_ADDR_EXT (1 << 5)
    mov eax, cr4
    or eax, CR4_PAGE_SIZE_EXT | CR4_PHYS_ADDR_EXT
    mov cr4, eax

    ; enable long mode
    mov ecx, 0xc0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ; load 64 bit GDT
    lgdt [EARLY_PHYS(gdtr)]

    ; reload code segment
    jmp 0x08:EARLY_PHYS(long_mode)

bits 64
long_mode:
    ; setup pml4 entry for pdpt_k
    mov rax, EARLY_PHYS(pdpt_k)
    or rax, PAGE_PRESENT | PAGE_WRITABLE
    mov rbx, EARLY_PHYS(pml4) + ((KERNEL_BASE >> 39) & 511) * 8
    mov [rbx], rax

    ; setup pdpt_k entry for pd_k
    mov rax, EARLY_PHYS(pd_k)
    or rax, PAGE_PRESENT | PAGE_WRITABLE
    mov rbx, EARLY_PHYS(pdpt_k) + ((KERNEL_BASE >> 30) & 511) * 8
    mov [rbx], rax

    ; setup pd_k entry for pt_k
    mov rax, EARLY_PHYS(pt_k)
    or rax, PAGE_PRESENT | PAGE_WRITABLE
    mov rbx, EARLY_PHYS(pd_k) + ((KERNEL_BASE >> 21) & 511) * 8
    mov [rbx], rax

    ; map kernel in pt k
    mov rdi, EARLY_PHYS(pt_k)
    mov rsi, _base
    mov r8, _rodata_end
    mov r9, _end
.map_kernel:
    ; build pt k entry
    mov rax, rsi
    mov rbx, KERNEL_BASE - KERNEL_PHYS_BASE
    sub rax, rbx
    or rax, PAGE_PRESENT
    cmp rsi, r8
    jb .no_write
    or rax, PAGE_WRITABLE
.no_write:
    stosq
    ; compare to end
    add rsi, PAGE_SIZE
    cmp rsi, r9
    jb .map_kernel

    ; jump to kernel in higher half
    mov rax, higher_half
    jmp rax

higher_half:
    ; unmap stack guard
    mov rbx, stackguard
    mov rax, 0x0000ffffffffffff
    and rbx, rax
    shr rbx, 12
    shl rbx, 3 ; * 8
    mov rax, PAGE_TABLES
    add rbx, rax
    mov [rbx], dword 0

    ; set up kernel stack
    mov rsp, stackend

    ; init phys allocator
    mov rsi, [EARLY_MEMORY_MAP_LEN]
    mov rdi, EARLY_MEMORY_MAP
    call phys_init_regions

    ; unmap low memory
    xor rax, rax
    mov rbx, EARLY_PHYS(pml4)
    mov [rbx], rax

    ; flush TLB
    mov rax, cr3
    mov cr3, rax

    ; point tss base to tss
    ; mov eax, tss
    ; mov [gdt.tss_base_0_15], ax
    ; shr eax, 16
    ; mov [gdt.tss_base_16_23], al
    ; mov [gdt.tss_base_24_31], ah

    ; reload GDT in high memory
    mov rax, EARLY_PHYS(gdtr)
    lgdt [rax]

    ; load tss
    mov ax, SEG_TSS
    ltr ax

    ; initialize interrupts
    call isrs_init

    ; enable interrupts
    sti

    push 0
    jmp main

section .data
gdtr:
    dw (gdt.end - gdt) - 1 ; size
    dd EARLY_PHYS(gdt)     ; offset

gdt:
    ; null entry
    dq 0
    ; code entry
    dq GDT64_DESCRIPTOR | GDT64_PRESENT | GDT64_READWRITE | GDT64_EXECUTABLE | GDT64_64BIT
    ; data entry
    dq GDT64_DESCRIPTOR | GDT64_PRESENT | GDT64_READWRITE
.end:

; TODO - 64 bit tss
; global tss
; tss:
;     dd 0            ; link
;     dd stackend     ; esp0
;     dd SEG_KDATA    ; ss0
;     times (TSS_IOPB_OFFSET - (4 * 3)) db 0 ; skip unused fields
;     dw 0            ; reserved
;     dw TSS_SIZE     ; iopb offset

section .bss
    align PAGE_SIZE
    pml4    resb PAGE_SIZE
    ; early huge 4 MiB identity mapping to get us rolling:
    pdpt_0  resb PAGE_SIZE
    pd_0    resb PAGE_SIZE
    ; higher half kernel mapping tables:
    pdpt_k  resb PAGE_SIZE
    pd_k    resb PAGE_SIZE
    pt_k    resb PAGE_SIZE

    memory_map      resb PAGE_SIZE

    global temp_page
    temp_page   resb PAGE_SIZE

section .stack
    global stackguard
    align PAGE_SIZE
    stackguard times PAGE_SIZE db 0
    global stack
    stack times 8 * PAGE_SIZE db 0
    global stackend
    stackend equ $
