org 0x8000
use16

    %include "kernel/src/consts.asm"

    ; enable A20 line
    mov ax, 0x2401
    int 0x15
    mov si, could_not_enable_a20
    jc error_bios

    ; init memory map len
    mov [EARLY_MEMORY_MAP_LEN], dword 0

    ; read memory map
    mov di, EARLY_MEMORY_MAP
    xor ebx, ebx
.memory_map_loop:
    mov edx, 0x534d4150
    mov eax, 0xe820
    mov ecx, 24
    int 0x15
    jc .memory_map_done
    test ebx, ebx
    jz .memory_map_done
    add di, 24
    inc dword [EARLY_MEMORY_MAP_LEN]
    cmp di, EARLY_MEMORY_MAP_END
    jb .memory_map_loop
.memory_map_done:

    ; search for KERNEL.2 in root dir on disk
    mov ax, kernel2_filename
    call fat_find_file

cluster_read_loop:
    ; set up bufseg
    mov [lbapkt.bufseg], word read_buffer >> 4

    ; read cluster
    call fat_read_next_cluster

    ; load GDT for unreal mode
    cli
    sgdt [bios_gdtr]
    lgdt [gdtr32]

    ; set protected mode bit of cr0
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; set up base of ES segment
    mov eax, [kernel_read_ptr]
    mov [gdt32.data_base_0_w], ax
    shr eax, 16
    mov [gdt32.data_base_16_b], al
    mov [gdt32.data_base_24_b], ah

    ; load unreal selector into es
    mov ax, SEG_KDATA
    mov es, ax

    ; copy from read buffer into place
    movzx ecx, byte [fatctx_sectors_per_cluster]
    shl ecx, 9 ; 512 bytes per sector
    add [kernel_read_ptr], ecx
    shr ecx, 2 ; 4 bytes per dword
    mov esi, read_buffer
    xor edi, edi
    rep movsd

    ; exit protected mode
    mov eax, cr0
    and eax, ~1
    mov cr0, eax

    ; reload es
    mov ax, 0
    mov es, ax

    ; restore bios gdt
    lgdt [bios_gdtr]
    sti

    ; loop around
    cmp [fatctx_current_cluster], word 0xfff7
    jb cluster_read_loop

loaded:
    ; print starting message
    mov si, starting
    call print

    ; reset base of data segment descriptor
    xor ax, ax
    mov [gdt32.data_base_0_w], ax
    mov [gdt32.data_base_16_b], al
    mov [gdt32.data_base_24_b], al

    ; set up 32 bit GDT and null IDT
    cli
    lgdt [gdtr32]
    lidt [idtr32]

    ; set protected mode bit of cr0
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; far jump to protected mode. this time we're not going back
    mov eax, LOADER_2_BASE
    jmp SEG_KCODE:trampoline

trampoline:
    db 0xff, 0xe0 ; jmp eax

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

%define FATCTX_PTR fatctx
%include "kernel/loader/fat.asm"

could_not_enable_a20 db "Could not enable A20 line", 0
could_not_read_memory_map db "Could not read memory map from BIOS", 0

starting db "Starting...", 0
kernel2_filename db "KERNEL  2  "

read_buffer equ 0x2000

kernel_read_ptr dd LOADER_2_BASE

bios_gdtr:
    dw 0
    dd 0

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
    .data_base_0_w:
    dw 0x0000       ; base 0:15
    .data_base_16_b:
    db 0x00         ; base 16:23
    db 0b10010010   ; access byte - data
    db 0xcf         ; flags/(limit 16:19). 4 KB granularity + 32 bit mode flags
    .data_base_24_b:
    db 0x00         ; base 24:31
.end:

idtr32:
    dw 0
    dd 0
