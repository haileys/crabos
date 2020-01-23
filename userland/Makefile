.PHONY: all
all: target/bin/init.bin

.PHONY: clean
clean:
	rm -rf target

target/bin/init.bin: linker.ld target/crt0.o target/init.o
	@mkdir -p target/bin
	x86_64-elf-ld -o $@ -T $^

target/%.o: %.asm
	@mkdir -p target
	nasm -f elf64 -o $@ $<
