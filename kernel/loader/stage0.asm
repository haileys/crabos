org 0x7c00
use16

    %include "kernel/src/consts.asm"

    ; initialize cs
    jmp 0:start
start:
    ; initialize stack
    mov sp, 0x7c00

    ; initialise FAT routines
    call fat_init

    ; search for KERNEL.1 in root dir on disk
    mov ax, kernel1_filename
    call fat_find_file

    ; read next cluster into place
cluster_read_loop:
    ; set up bufseg
    mov ax, [kernel_seg]
    mov [lbapkt.bufseg], ax

    ; read cluster
    call fat_read_next_cluster

    ; advance kernel seg
    movzx ax, byte [fatctx_sectors_per_cluster]
    shl ax, 5 ; bytes_per_cluster = sectors_per_cluster * 512
              ; segment_per_cluster = bytes_per_cluster / 16
              ;                     = sectors_per_cluster * 32
              ;                     = sectors_per_cluster << 5
    add [kernel_seg], ax

    ; loop around
    cmp [fatctx_current_cluster], word 0xfff7
    jb cluster_read_loop

loaded:
    ; print starting message
    mov si, starting
    call print

    ; jump to kernel start
    jmp LOADER_1_BASE

%define FATCTX_PTR fatctx
%include "kernel/loader/fat.asm"

starting db "Loading kernel...", 13, 10, 0

kernel1_filename db "KERNEL  1  "

kernel_seg dw LOADER_1_BASE >> 4

times 0x1be - ($ - $$) db 0
