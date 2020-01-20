#!/bin/bash -e

cd "$(dirname "$0")"

AUTOCONF=autoconf-2.64
AUTOMAKE=automake-1.12
PREFIX="$(pwd)/autotools"

if [ ! -f "tmp/$AUTOCONF.tar.xz" ]; then
    echo "+++ Downloading autoconf..."
    (
        mkdir -p tmp
        cd tmp
        curl -O "https://ftp.gnu.org/gnu/autoconf/$AUTOCONF.tar.xz"
        tar xf "$AUTOCONF.tar.xz"
    )
fi

if [ ! -f "tmp/$AUTOMAKE.tar.xz" ]; then
    echo "+++ Downloading automake..."
    (
        mkdir -p tmp
        cd tmp
        curl -O "https://ftp.gnu.org/gnu/automake/$AUTOMAKE.tar.xz"
        tar xf "$AUTOMAKE.tar.xz"
    )
fi

echo "+++ Building autoconf..."
(
    cd "tmp/$AUTOCONF"
    ./configure --prefix="$PREFIX"
    make -j 8
    make install
)

echo "+++ Building automake..."
(
    cd "tmp/$AUTOMAKE"
    ./configure --prefix="$PREFIX"
    make -j 8
    make install
)
