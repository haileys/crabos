use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        asm!("xchgw %bx, %bx" :: "rax"(0xf0f0f0f0_f0f0f0f0u64));
    }

    loop {}
}
