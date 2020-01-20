#!/bin/bash -e

cd "$(dirname "$0")"

BINUTILS=binutils-2.33.1
PREFIX="$(pwd)/cross-elf"

if [ ! -f "tmp/$BINUTILS.tar.xz" ]; then
    echo "+++ Downloading binutils"
    mkdir -p tmp
    (
        cd tmp
        curl -O "https://ftp.gnu.org/gnu/binutils/$BINUTILS.tar.xz"
    )
fi

if  [ ! -d "tmp/$BINUTILS" ]; then
    echo "+++ Extracting binutils"
    (
        cd tmp
        tar xf "$BINUTILS".tar.xz
    )

    echo "+++ Patching binutils"
    (
        cd "tmp/$BINUTILS"
        patch -p0 < "../../$BINUTILS.patch"
    )
fi

echo "+++ Building binutils"

cd tmp
rm -rf build-elf-binutils
mkdir build-elf-binutils
cd build-elf-binutils

"../$BINUTILS/configure" --target=x86_64-elf --disable-werror --prefix="$PREFIX"
make -j 8
make install
