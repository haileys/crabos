use32

%include "kernel/src/consts.asm"

extern _base
extern _bss
extern _rodata_end
extern _bss_end
extern _tls_end
extern main
extern phys_init
extern isrs_init
extern console_init

global start

; 16 bit start code:
start:
    incbin "target/loader/stage2.bin"

; the kernel is linked with base = 0xffff800000000000, but the early bootloader
; places us at 0x8000 without paging enabled. we need to be careful in this
; early phase to translate symbol addresses to physical addresses:
%define EARLY_PHYS(addr) ((addr) - KERNEL_BASE + KERNEL_PHYS_BASE)

bits 32
protected_mode:
    ; zero bss pages
    xor eax, eax
    mov edi, EARLY_PHYS(_bss)
    mov ecx, EARLY_PHYS(_bss_end)
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
    mov eax, EARLY_PHYS(gdtr)
    lgdt [eax]

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
    mov r9, _bss_end
    mov r10, stackguard
map_kernel:
    ; don't map stack guard:
    cmp rsi, r10
    je .commit
    ; build pt k entry:
    mov rax, rsi
    mov rbx, KERNEL_BASE - KERNEL_PHYS_BASE
    sub rax, rbx
    or rax, PAGE_PRESENT
    ; don't map ro sections as writable:
    cmp rsi, r8
    jb .commit
    or rax, PAGE_WRITABLE
.commit:
    stosq
    ; compare to end
    add rsi, PAGE_SIZE
    cmp rsi, r9
    jb map_kernel

    ; flush TLB
    mov rax, cr3
    mov cr3, rax

    ; jump to kernel in higher half
    mov rax, higher_half
    jmp rax

higher_half:
    ; set up kernel stack
    mov rsp, stackend

    ; set up TLS
    mov ecx, 0xc0000100 ; MSR_FS_BASE
    mov rax, tcb
    mov rdx, rax
    shr rdx, 32
    wrmsr

    ; init phys allocator
    mov rdi, EARLY_MEMORY_MAP
    mov rsi, [EARLY_MEMORY_MAP_LEN]
    mov rdx, EARLY_PHYS(_bss_end)
    call phys_init

    ; init console
    mov rdi, EARLY_VBE_MODE_INFO
    mov rsi, EARLY_BIOS_FONT
    call console_init

    ; unmap low memory
    xor rax, rax
    mov rbx, EARLY_PHYS(pml4)
    mov [rbx], rax

    ; flush TLB
    mov rax, cr3
    mov cr3, rax

    ; point tss gdt entry to tss
    mov [rel gdt.tss_size_0_15], word tss.end - tss
    mov rax, tss
    mov [rel gdt.tss_base_0_15], ax
    shr rax, 16
    mov [rel gdt.tss_base_16_23], al
    shr rax, 8
    mov [rel gdt.tss_base_24_31], al
    shr rax, 8
    mov [rel gdt.tss_base_32_63], eax

    ; reload GDT in high memory
    mov rax, qword gdt
    mov [rel gdtr.offset], rax
    lgdt [rel gdtr]

    ; load tss
    mov ax, SEG_TSS
    ltr ax

    ; initialize interrupts
    call isrs_init

    ; interrupts are disabled for main, but enabled as soon as task::start
    ; irets to first task

    push 0
    jmp main

section .data
gdtr:
    .size   dw (gdt.end - gdt) - 1 ; size
    .offset dq EARLY_PHYS(gdt)     ; offset

gdt:
    ; null entry
    dq 0
    ; kernel code entry
    dq GDT64_DESCRIPTOR | GDT64_PRESENT | GDT64_READWRITE | GDT64_EXECUTABLE | GDT64_64BIT
    ; kernel data entry
    dq GDT64_DESCRIPTOR | GDT64_PRESENT | GDT64_READWRITE
    ; user code entry
    dq GDT64_DESCRIPTOR | GDT64_PRESENT | GDT64_READWRITE | GDT64_EXECUTABLE | GDT64_64BIT | GDT64_USER
    ; user data entry
    dq GDT64_DESCRIPTOR | GDT64_PRESENT | GDT64_READWRITE | GDT64_USER
    ; tss entry
    .tss_size_0_15  dw 0
    .tss_base_0_15  dw 0
    .tss_base_16_23 db 0
    .tss_access     db 0x89
    .tss_flags_lim  db (1 << 4)
    .tss_base_24_31 db 0
    .tss_base_32_63 dd 0
    .tss_reserved   dd 0
.end:

tcb:
    ; tcb+0 points to end of TLS block
    dq _tls_end

global tss
tss:
    dd 0                ; reserved
    dq stackend         ; rsp0
    dq 0                ; rsp1
    dq 0                ; rsp2
    dq 0                ; reserved
    dq 0                ; ist1
    dq 0                ; ist2
    dq 0                ; ist3
    dq 0                ; ist4
    dq 0                ; ist5
    dq 0                ; ist6
    dq 0                ; ist7
    dq 0                ; reserved
    dw 0                ; reserved
    dw (tss.end - tss)  ; iopb offset
.end:

section .bss
    align PAGE_SIZE
    pml4        resb PAGE_SIZE
    ; early huge 4 MiB identity mapping to get us rolling:
    pdpt_0      resb PAGE_SIZE
    pd_0        resb PAGE_SIZE
    ; higher half kernel mapping tables:
    pdpt_k      resb PAGE_SIZE
    pd_k        resb PAGE_SIZE
    pt_k        resb PAGE_SIZE

    memory_map  resb PAGE_SIZE

    global temp_page
    temp_page   resb PAGE_SIZE

section .stack
    global stackguard
    align PAGE_SIZE
    stackguard times PAGE_SIZE db 0
    global stack
    stack times 16 * PAGE_SIZE db 0
    global stackend
    stackend equ $
