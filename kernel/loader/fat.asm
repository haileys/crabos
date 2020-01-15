fat_init:
    ; check partition 1 is bootable
    mov al, [PART1 + PART_STATUS]
    bt ax, 7
    jnc err_no_bootable_part

    ; load MBR of partition 1
    mov eax, [PART1 + PART_LBA_FIRST]
    mov [lba_read.bufseg], word part_mbr >> 4
    mov [lba_read.lbalo], eax
    call lba_read

    ; calculate first fat sector
    movzx eax, word [reserved_sectors]
    add eax, dword [PART1 + PART_LBA_FIRST]
    mov [first_fat_sector], eax

    ; calculate first data sector according to FAT BPB parameters
    movzx ax, byte [fat_count]
    mul word [sectors_per_fat] ; DX:AX = AX * sectors_per_fat
    shl edx, 16
    mov dx, ax ; EDX = fat_count * sectors_per_fat
    add edx, [first_fat_sector]
    mov dword [first_data_sector], edx

    ret

err_no_bootable_part:
    mov si, .msg
    call print
    jmp $
    .msg db "ERR#0", 0

print:
    lodsb
    or al, al
    jz .end
    mov ah, 0x0e
    int 0x10
    jmp print
.end:
    ret
