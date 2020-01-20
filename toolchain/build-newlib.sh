#!/bin/bash -e

cd "$(dirname "$0")"

SYSROOT="$(pwd)/sysroot"

mkdir -p tmp
cd tmp

# newlib is particular about the toolchain used to build it. we need to
# masquerade our freestanding elf toolchain as a crabos toolchain to
# bootstrap newlib:
echo "+++ Setting up cross compiler symlinks"
mkdir -p build-newlib-toolchain
(
    cd build-newlib-toolchain
    mkdir -p bin
    ln -f `which x86_64-elf-ar` bin/x86_64-crabos-ar
    ln -f `which x86_64-elf-as` bin/x86_64-crabos-as
    ln -f `which x86_64-elf-gcc` bin/x86_64-crabos-gcc
    ln -f `which x86_64-elf-gcc` bin/x86_64-crabos-cc
    ln -f `which x86_64-elf-ranlib` bin/x86_64-crabos-ranlib
    ln -sf "$(dirname "$(dirname "$(which x86_64-elf-gcc)")")"/lib lib
    ln -sf "$(dirname "$(dirname "$(which x86_64-elf-gcc)")")"/libexec libexec
)

echo "+++ Building newlib"

export PATH="$(pwd)/build-newlib-toolchain/bin:$PATH"

rm -rf build-newlib
mkdir build-newlib
(
    cd build-newlib
    ../../newlib/configure --prefix=/usr --target=x86_64-crabos --enable-newlib-io-long-long
    make -j 8 all
    make DESTDIR="$SYSROOT" install
)

ln -sf "$SYSROOT"/usr/x86_64-crabos/include "$SYSROOT"/usr/include
ln -sf "$SYSROOT"/usr/x86_64-crabos/lib "$SYSROOT"/usr/lib
