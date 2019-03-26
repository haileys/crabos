%define PHYS_ALLOC_START        0x00100000

%define KERNEL_BASE             0xc0000000
%define KERNEL_PHYS_BASE        0x00008000

%define PAGE_SIZE               4096
%define PAGE_PRESENT            (1 << 0)
%define PAGE_WRITABLE           (1 << 1)

%define EARLY_MEMORY_MAP        0x00004000
%define EARLY_MEMORY_MAP_END    0x00004ff0
%define EARLY_MEMORY_MAP_LEN    0x00004ff0

%define PAGE_TABLES             0xffc00000
