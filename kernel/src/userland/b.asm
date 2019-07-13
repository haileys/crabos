bits 64

    mov dl, '.'
L:
    int 0x7f
    jmp L
