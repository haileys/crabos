; expects definitions: FATCTX_PTR

%define SECTOR_SIZE 512
%define ENTRY_FIRST_CLUSTER 0x1a

fat_init:
    ; save boot device
    mov [fatctx_boot_device], dl

    ; check partition 1 is bootable
    mov al, [PART1 + PART_STATUS]
    bt ax, 7
    jnc err_no_bootable_part

    ; load MBR of partition 1
    mov eax, [PART1 + PART_LBA_FIRST]
    mov [lbapkt.bufseg], word fatctx_mbr >> 4
    mov [lbapkt.lbalo], eax
    call lba_read

    ; calculate first fat sector
    movzx eax, word [fatctx_reserved_sectors]
    add eax, dword [PART1 + PART_LBA_FIRST]
    mov [fatctx_first_fat_sector], eax

    ; calculate first data sector according to FAT BPB parameters
    movzx ax, byte [fatctx_fat_count]
    mul word [fatctx_sectors_per_fat] ; DX:AX = AX * sectors_per_fat
    shl edx, 16
    mov dx, ax ; EDX = fat_count * sectors_per_fat
    add edx, [fatctx_first_fat_sector]
    mov dword [fatctx_first_data_sector], edx

    ; edx now contains first data sector
    ; read root directory from disk
    mov [lbapkt.lbalo], edx
    mov [lbapkt.bufseg], word fatctx_root_directory >> 4
    call lba_read

    ret

; Params:  AX = near pointer to kernel filename
fat_find_file:
    ; search for KERNEL in root dir on disk
    mov bx, fatctx_root_directory
.search_file:
    ; compare 11 byte name field
    mov cx, 11
    mov si, bx
    mov di, ax ; kernel filename
    repe cmpsb
    je .found_file
    ; try next entry
    add bx, 0x20
    cmp bx, fatctx_root_directory + 0x200
    jb .search_file
.err_no_kernel_file:
    mov si, .msg
    call print
    jmp $
    .msg db "Missing system file", 0
.found_file:
    ; load first cluster from dir entry
    mov ax, [bx + ENTRY_FIRST_CLUSTER]
    mov [fatctx_current_cluster], ax
    ret

; reads cluster in fatctx_current_cluster into lbapkt.buf*
fat_read_next_cluster:
    ;
    ; convert cluster number to LBA
    ;

    mov ax, [fatctx_current_cluster]
    sub ax, 2 ; cluster numbers are 2 indexed for some reason?
    movzx dx, [fatctx_sectors_per_cluster]
    mul dx

    ; set lbapkt.lbalo to sectors_per_cluster * (cluster_num - 2)
    mov [lbapkt.lbalo], ax
    mov [lbapkt.lbalo + 2], dx

    ; add root directory sector count to lbalo
    movzx eax, word [fatctx_root_dir_entries]
    shr eax, 4 ; div by 16

    ; add first_data_sector
    add eax, [fatctx_first_data_sector]

    ; add to lbapkt.lbalo
    add [lbapkt.lbalo], eax

    ;
    ; read cluster
    ;

    ; set lba sector count
    movzx eax, byte [fatctx_sectors_per_cluster]
    mov [lbapkt.count], ax

    ; do read
    call lba_read

    ; read fat sector corresponding to current cluster
    movzx eax, byte [fatctx_current_cluster + 1]
    add eax, [fatctx_first_fat_sector]
    mov [lbapkt.count], word 1
    mov [lbapkt.lbalo], eax
    mov [lbapkt.bufseg], word fatctx_fat_sector >> 4
    call lba_read

    ; index fat sector by current_cluster & 0x00ff
    movzx bx, byte [fatctx_current_cluster]
    shl bx, 1 ; bx * 2
    mov ax, [fatctx_fat_sector + bx]

    ; update fatctx_current_cluster
    mov [fatctx_current_cluster], ax

    ret

err_no_bootable_part:
    mov si, .msg
    call print
    jmp $
    .msg db "No boot partition", 0

print:
    lodsb
    or al, al
    jz .end
    mov ah, 0x0e
    int 0x10
    jmp print
.end:
    ret

lba_read:
    ; perform LBA read
    mov si, lbapkt
    mov ah, 0x42
    mov dl, [fatctx_boot_device]
    int 0x13
    ; check error
    jc .err
    ret
.err:
    mov si, .msg
    call print
    jmp $
.msg:
    db "Disk read error", 0
    align 4

;;; DATA

lbapkt:
    .size   db 16
    .res    db 0
    .count  dw 1
    .bufoff dw 0
    .bufseg dw 0
    .lbalo  dd 1
    .lbahi  dd 0

fatctx                      equ 0x500

fatctx_boot_device          equ 0x0500 ; byte
fatctx_current_cluster      equ 0x0502 ; word
fatctx_first_data_sector    equ 0x0504 ; dword
fatctx_first_fat_sector     equ 0x0508 ; dword

fatctx_mbr                  equ 0x0600 ; 512 bytes
fatctx_sectors_per_cluster  equ fatctx_mbr + 0x0d ; byte
fatctx_reserved_sectors     equ fatctx_mbr + 0x0e ; word
fatctx_fat_count            equ fatctx_mbr + 0x10 ; byte
fatctx_root_dir_entries     equ fatctx_mbr + 0x11 ; word
fatctx_sectors_per_fat      equ fatctx_mbr + 0x16 ; word

fatctx_fat_sector           equ 0x0800
fatctx_root_directory       equ 0x1000

PART1                       equ 0x7c00 + 0x1be
; we won't search for partitions at current:
; PART2 equ 0x7c00 + 0x1ce
; PART3 equ 0x7c00 + 0x1de
; PART4 equ 0x7c00 + 0x1fe

PART_STATUS                 equ 0x00
PART_TYPE                   equ 0x04
PART_LBA_FIRST              equ 0x08
PART_SECCOUNT               equ 0x0c
PART_LEN                    equ 0x10
