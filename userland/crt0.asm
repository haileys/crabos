global _start
global syscall_alloc_page
global syscall_release_page
global syscall_modify_page
global syscall_release_handle
global syscall_clone_handle
global syscall_debug
global syscall_set_page_context
global syscall_get_page_context
global syscall_create_task
global syscall_exit
global syscall_map_physical_memory

extern main

%define STACK_TOP   0x80000000
%define STACK_SIZE  (64 * 1024)
%define PAGE_SIZE   4096
%define PAGE_WRITE  2

_start:
    ; allocate stack
    mov rdi, STACK_TOP - STACK_SIZE
    mov rsi, STACK_SIZE / PAGE_SIZE
    mov rdx, PAGE_WRITE
    mov rax, 1 ; SYSCALL_ALLOC_PAGE
    int 0x7f
    ; assume that works for now, TODO check error

    ; setup stack
    mov rsp, STACK_TOP

    call main

    mov rdi, rax
    call syscall_exit

syscall_alloc_page:
    mov rax, 1
    int 0x7f
    ret

syscall_release_page:
    mov rax, 2
    int 0x7f
    ret

syscall_modify_page:
    mov rax, 3
    int 0x7f
    ret

syscall_release_handle:
    mov rax, 4
    int 0x7f
    ret

syscall_clone_handle:
    mov rax, 5
    int 0x7f
    ret

syscall_create_page_context:
    mov rax, 6
    int 0x7f
    ret

syscall_debug:
    mov rax, 7
    int 0x7f
    ret

syscall_set_page_context:
    mov rax, 8
    int 0x7f
    ret

syscall_get_page_context:
    mov rax, 9
    int 0x7f
    ret

syscall_create_task:
    mov rax, 10
    int 0x7f
    ret

syscall_exit:
    mov rax, 11
    int 0x7f
    ret

syscall_map_physical_memory:
    mov rax, 12
    int 0x7f
    ret
