BUILD?=debug

KERNEL_ELF=target/x86_64-kernel/$(BUILD)/kernel
KERNEL_BIN=target/x86_64-kernel/$(BUILD)/kernel.bin
KERNEL_OBJS=\
	target/x86_64-kernel/start.o \
	target/x86_64-kernel/isrs.o \
	target/x86_64-kernel/aux.o \
	target/x86_64-kernel/userland/a.bin \
	target/x86_64-kernel/userland/b.bin \

ifeq ($(BUILD),release)
CARGO_FLAGS=--release
endif

default: hdd.img

.PHONY: clean
clean:
	rm -f hdd.img
	rm -f target/loader/stage0.bin
	rm -f target/loader/stage1.bin
	rm -f target/x86_64-kernel/start.o
	cargo clean

hdd.img: hdd.base.img target/loader/stage0.bin $(KERNEL_BIN)
	cp hdd.base.img hdd.img
	MTOOLSRC=mtoolsrc mformat C:
	MTOOLSRC=mtoolsrc mcopy $(KERNEL_BIN) C:/KERNEL
	dd if=target/loader/stage0.bin of=$@ bs=446 count=1 conv=notrunc,sync

$(KERNEL_BIN): $(KERNEL_ELF)
	x86_64-elf-objcopy -R .bss -R .stack -O binary $(KERNEL_ELF) $(KERNEL_BIN)

.PHONY: $(KERNEL_ELF)
$(KERNEL_ELF):  kernel/linker.ld $(KERNEL_OBJS)
	cargo xbuild --target=kernel/x86_64-kernel.json $(CARGO_FLAGS)

target/loader/stage0.bin: kernel/loader/stage0.asm kernel/src/consts.asm
	mkdir -p target/loader
	nasm -f bin -o $@ $<

target/loader/stage1.bin: kernel/loader/stage1.asm kernel/src/consts.asm
	mkdir -p target/loader
	nasm -f bin -o $@ $<

target/x86_64-kernel/%.o: kernel/src/%.asm kernel/src/consts.asm target/loader/stage1.bin
	mkdir -p target/x86_64-kernel
	nasm -f elf64 -o $@ $<

target/x86_64-kernel/%.bin: kernel/src/%.asm
	mkdir -p $$(dirname '$@')
	nasm -f bin -o $@ $<
