org 0x7c00
use16

    %include "kernel/src/consts.asm"

    ; initialize cs
    jmp 0:start
start:
    ; initialize stack
    mov sp, 0x7c00

    ; save boot device
    mov [boot_device], dl

    ; initialise FAT routines
    call fat_init

    ; edx now contains first data sector
    ; read root directory from disk
    mov [lba_read.lbalo], edx
    mov [lba_read.bufseg], word root_directory >> 4
    call lba_read

    ; search for KERNEL in root dir on disk
    mov bx, root_directory
search_file:
    ; compare 11 byte name field
    mov cx, 11
    mov si, bx
    mov di, kernel_filename
    repe cmpsb
    je found_file
    ; try next entry
    add bx, 0x20
    cmp bx, 0x200
    jb search_file
    mov si, no_kernel_file

found_file:
    ; load first cluster from dir entry
    mov ax, [bx + ENTRY_FIRST_CLUSTER]
    mov [current_cluster], ax

cluster_read:
    ; find cluster number
    call cluster_to_lba
    mov eax, [lba_read.lbalo]
    ; read cluster into place
    mov ax, [kernel_seg]
    mov [lba_read.bufseg], ax
    ; set lba sector count
    movzx eax, byte [sectors_per_cluster]
    mov [lba_read.count], ax
    ; do read
    call lba_read
    ; advance kernel seg
    movzx ax, byte [sectors_per_cluster]
    shl ax, 5 ; bytes_per_cluster = sectors_per_cluster * 512
              ; segment_per_cluster = bytes_per_cluster / 16
              ;                     = sectors_per_cluster * 32
              ;                     = sectors_per_cluster << 5
    add [kernel_seg], ax

    ; read fat sector corresponding to current cluster
    movzx eax, byte [current_cluster + 1]
    add eax, [first_fat_sector]
    mov [lba_read.count], word 1
    mov [lba_read.lbalo], eax
    mov [lba_read.bufseg], word fat_sector >> 4
    call lba_read

    ; index fat sector by current_cluster & 0x00ff
    movzx bx, byte [current_cluster]
    shl bx, 1 ; bx * 2
    mov ax, [fat_sector + bx]
    mov [current_cluster], ax
    cmp ax, 0xfff7
    jb cluster_read ; read next cluster in chain if valid link

loaded:
    ; print starting message
    mov si, starting
    call print

    ; jump to kernel start
    jmp KERNEL_PHYS_BASE

%include "kernel/loader/fat.asm"

; takes cluster number in AX, returns LBA in lba_read.lbalo
cluster_to_lba:
    ; preserve dx
    push ax
    push dx

    sub ax, 2 ; cluster numbers are 2 indexed for some reason?
    movzx dx, [sectors_per_cluster]
    mul dx
    ; set lba_read.lbalo to sectors_per_cluster * (cluster_num - 2)
    mov [lba_read.lbalo], ax
    mov [lba_read.lbalo + 2], dx
    ; add root directory sector count to lbalo
    movzx eax, word [root_dir_entries]
    shr eax, 4 ; div by 16
    ; add first_data_sector
    add eax, [first_data_sector]
    ; add to lba_read.lbalo
    add [lba_read.lbalo], eax

    pop dx
    pop ax
    ret

no_kernel_file:
    mov si, .msg
    call print
    jmp $
    .msg db "ERR#1", 0

lba_read:
    ; perform LBA read
    mov si, .pkt
    mov ah, 0x42
    mov dl, [boot_device]
    int 0x13

    ; check error
    jc .err

    ret

.err:
    mov si, .msg
    call print
    jmp $

.msg:
    db "ERR#3", 0

    align 4
.pkt:
    .size   db 16
    .res    db 0
    .count  dw 1
    .bufoff dw 0
    .bufseg dw 0
    .lbalo  dd 1
    .lbahi  dd 0

starting db "Starting...", 0

kernel_filename db "KERNEL     "

kernel_seg dw KERNEL_PHYS_BASE >> 4

times 0x1be - ($ - $$) db 0

SECTOR_SIZE         equ 512

boot_device         equ 0x0500 ; byte
current_cluster     equ 0x0502 ; word
first_data_sector   equ 0x0504 ; dword
first_fat_sector    equ 0x0508 ; dword

part_mbr            equ 0x0600 ; 512 bytes
sectors_per_cluster equ part_mbr + 0x0d ; byte
reserved_sectors    equ part_mbr + 0x0e ; word
fat_count           equ part_mbr + 0x10 ; byte
root_dir_entries    equ part_mbr + 0x11 ; word
sectors_per_fat     equ part_mbr + 0x16 ; word

fat_sector          equ 0x0800
root_directory      equ 0x1000

ENTRY_FIRST_CLUSTER equ 0x1a

PART1 equ 0x7c00 + 0x1be
; we won't search for partitions at current:
; PART2 equ 0x7c00 + 0x1ce
; PART3 equ 0x7c00 + 0x1de
; PART4 equ 0x7c00 + 0x1fe

PART_STATUS     equ 0x00
PART_TYPE       equ 0x04
PART_LBA_FIRST  equ 0x08
PART_SECCOUNT   equ 0x0c
PART_LEN        equ 0x10
