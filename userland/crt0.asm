global _start
extern main_
extern syscall_alloc_page
extern syscall_exit

%define STACK_TOP   0x80000000
%define STACK_SIZE  (64 * 1024)
%define PAGE_SIZE   4096
%define PAGE_WRITE  2

_start:
    xchg bx, bx

    ; allocate stack
    mov rdi, STACK_TOP - STACK_SIZE
    mov rsi, STACK_SIZE / PAGE_SIZE
    mov rdx, PAGE_WRITE
    mov rax, 1 ; SYSCALL_ALLOC_PAGE
    int 0x7f
    ; assume that works for now, TODO check error

    ; setup stack
    mov rsp, STACK_TOP

    call main_

    mov rdi, rax
    call syscall_exit
