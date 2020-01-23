global main

extern syscall_map_physical_memory

main:
    mov rdi, 0xe0000000
    mov rsi, 0xe0000000
    mov rdx, 8 * 1024 * 1024 / 4096
    mov rcx, 2 ; WRITE
    call syscall_map_physical_memory

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
