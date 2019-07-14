bits 64

    xchg bx, bx

    mov rax, 1          ; Syscall::AllocPage
    mov rdi, 0x13370000 ; virtual addr
    mov rsi, 16         ; page count
    mov rdx, 2          ; UserPageFlags::WRITE
    int 0x7f

    xchg bx, bx

    mov rax, 1          ; Syscall::AllocPage
    mov rdi, 0x23370000 ; virtual addr
    mov rsi, 4          ; page count
    mov rdx, 2          ; UserPageFlags::WRITE
    int 0x7f

    xchg bx, bx

L:
    jmp L
