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

%define SEG_KCODE               0x08
%define SEG_KDATA               0x10
%define SEG_TSS                 0x18

%define TSS_SIZE                0x68
%define TSS_IOPB_OFFSET         0x64

%define GDT64_DESCRIPTOR        (1 << 44)
%define GDT64_PRESENT           (1 << 47)
%define GDT64_READWRITE         (1 << 41)
%define GDT64_EXECUTABLE        (1 << 43)
%define GDT64_64BIT             (1 << 53)
