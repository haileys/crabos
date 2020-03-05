BUILD?=debug

ifeq ($(BUILD),release)
CARGO_FLAGS=--release
endif

.PHONY: all
all: init

.PHONY: clean
clean:
	rm -rf target

.PHONY: init
init: target/x86_64-crabos/crt0.o
	@mkdir -p target/bin
	cargo xbuild --target=x86_64-crabos.json $(CARGO_FLAGS)
	x86_64-elf-objcopy -R .bss -R .stack -O binary target/x86_64-crabos/$(BUILD)/init target/bin/init.bin

# target/bin/init.bin: linker.ld target/x86_64-crabos/crt0.o target/init.o
# 	@mkdir -p target/bin
# 	x86_64-elf-ld -o $@ -T $^

target/x86_64-crabos/%.o: %.asm
	@mkdir -p target/x86_64-crabos
	nasm -f elf64 -o $@ $<
