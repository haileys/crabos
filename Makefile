BUILD?=debug

KERNEL_ELF=target/i386-kernel/$(BUILD)/kernel
KERNEL_BIN=target/i386-kernel/$(BUILD)/kernel.bin
ifeq ($(BUILD),release)
CARGO_FLAGS=--release
endif

default: hdd.img

.PHONY: clean
clean:
	rm -f hdd.img
	rm -f target/loader/stage0.bin
	rm -f target/loader/stage1.bin
	rm -f target/i386-kernel/start.o
	cargo clean

hdd.img: target/loader/stage0.bin $(KERNEL_BIN)
	dd if=/dev/zero of=$@ bs=512 count=2048
	dd if=target/loader/stage0.bin of=$@ bs=512 count=1 conv=notrunc,sync
	dd if=$(KERNEL_BIN) of=$@ bs=512 seek=1 conv=notrunc,sync

$(KERNEL_BIN): $(KERNEL_ELF)
	i386-elf-objcopy -R .bss -R .stack -O binary $(KERNEL_ELF) $(KERNEL_BIN)

.PHONY: $(KERNEL_ELF)
$(KERNEL_ELF): target/i386-kernel/start.o kernel/linker.ld
	cargo xbuild --target=kernel/i386-kernel.json $(CARGO_FLAGS)

target/loader/stage0.bin: kernel/loader/stage0.asm kernel/src/consts.asm $(KERNEL_BIN)
	mkdir -p target/loader
	nasm -f bin -o $@ -DKERNEL_SIZE=$(shell stat -f %z $(KERNEL_BIN)) $<

target/loader/stage1.bin: kernel/loader/stage1.asm kernel/src/consts.asm
	mkdir -p target/loader
	nasm -f bin -o $@ $<

target/i386-kernel/start.o: kernel/src/start.asm kernel/src/consts.asm target/loader/stage1.bin
	mkdir -p target/i386-kernel
	nasm -f elf -o $@ $<
