%define LOADER_1_BASE           0x00008000
%define LOADER_2_BASE           0x00100000

%define KERNEL_PHYS_BASE        LOADER_2_BASE
%define KERNEL_BASE             0xffff800000000000

%define PAGE_SIZE               4096
%define PAGE_PRESENT            (1 << 0)
%define PAGE_WRITABLE           (1 << 1)
%define PAGE_HUGE               (1 << 7)

%define EARLY_MEMORY_MAP        0x00004000
%define EARLY_MEMORY_MAP_END    0x00004ff0
%define EARLY_MEMORY_MAP_LEN    0x00004ff0

%define SEG_KCODE               0x08
%define SEG_KDATA               0x10
%define SEG_UCODE               0x1b
%define SEG_UDATA               0x23
%define SEG_TSS                 0x28

%define TSS_SIZE                0x68
%define TSS_IOPB_OFFSET         0x64

%define GDT64_DESCRIPTOR        (1 << 44)
%define GDT64_PRESENT           (1 << 47)
%define GDT64_READWRITE         (1 << 41)
%define GDT64_EXECUTABLE        (1 << 43)
%define GDT64_64BIT             (1 << 53)
%define GDT64_USER              (3 << 45)
