#include <stdlib.h>

extern int main();

void _init_signal();

void _start() {
    _init_signal();
    main();

    __asm__ volatile ("xchgw %bx, %bx");
}
