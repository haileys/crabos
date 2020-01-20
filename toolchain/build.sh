#!/bin/bash -e

cd "$(dirname "$0")"

./build-autotools.sh
./build-elf-binutils.sh
./build-elf-gcc.sh
./build-newlib.sh
./build-hosted-binutils.sh
./build-hosted-gcc.sh
