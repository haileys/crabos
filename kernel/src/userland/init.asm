bits 64

    xor rax, rax
    lea rsi, [rel msg]
next:
    lodsb
    test al, al
    jz done
    int 0x7f
    jmp next
done:
    jmp $

msg db "Hello world from userland!", 0
