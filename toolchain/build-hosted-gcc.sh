#!/bin/bash -e

cd "$(dirname "$0")"

GCC=gcc-9.2.0
PREFIX="$(pwd)/cross-hosted"
TARGET="x86_64-crabos"
SYSROOT="$(pwd)/sysroot"

if [ ! -f "tmp/$GCC.tar.xz" ]; then
    echo "+++ Downloading gcc"
    (
        mkdir -p tmp
        cd tmp
        curl -O "https://ftp.gnu.org/gnu/gcc/$GCC/$GCC.tar.xz"
    )
fi

if [ ! -d "tmp/$GCC" ]; then
    echo "+++ Extracting gcc"
    (
        cd tmp
        tar xf "$GCC.tar.xz"
    )

    echo "+++ Patching gcc"
    (
        cd "tmp/$GCC"
        patch -p0 < "../../$GCC.patch"
    )

    echo "+++ Downloading gcc prerequisites"
    (
        cd "tmp/$GCC"
        ./contrib/download_prerequisites
    )
fi

echo "+++ Building gcc for $TARGET"
cd tmp
rm -rf build-hosted-gcc
mkdir build-hosted-gcc
cd build-hosted-gcc

PATH="$PREFIX/bin:$PATH" \
    "../$GCC/configure" \
    --prefix="$PREFIX" \
    --target="$TARGET" \
    --enable-languages=c,c++ \
    --disable-multilib \
    --with-sysroot="$SYSROOT"

make -j 8 all-gcc all-target-libgcc

make install-gcc install-target-libgcc
