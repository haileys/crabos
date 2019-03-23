%define PHYS_ALLOC_START    0x00100000

%define KERNEL_BASE         0xc0000000
%define KERNEL_PHYS_BASE    0x00008000

%define PAGE_SIZE       4096
%define PAGE_PRESENT    (1 << 0)
%define PAGE_WRITABLE   (1 << 1)
