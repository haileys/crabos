global main

extern syscall_map_physical_memory
extern syscall_release_page

main:
    ; map VBE mode info in low memory:
    mov rdi, 0x00006000
    mov rsi, 0x00006000
    mov rdx, 1
    mov rcx, 0 ; read only
    call syscall_map_physical_memory

    mov rdi, 0xe0000000
    ; read offset 40 of VBE mode info struct offset for phys_base
    mov rsi, [0x6000 + 40]
    mov rdx, 8 * 1024 * 1024 / 4096
    mov rcx, 2 ; WRITE
    call syscall_map_physical_memory

    ; unmap VBE mode info
    mov rdi, 0x6000
    mov rsi, 1
    call syscall_release_page

    mov r8, 0
    mov rdi, 0xe0000000
.lines:
    mov rax, r8
    xor rdx, rdx
    mov rbx, 3
    div rbx

    mov rbx, rax
    shl rax, 8
    or rax, rbx
    mov rbx, rax
    shl rax, 16
    or rax, rbx
    mov rbx, rax
    shl rax, 32
    or rax, rbx

    mov rcx, (1024 * 3) / 8
    rep stosq

    inc r8
    cmp r8, 768
    jb .lines

    jmp $
