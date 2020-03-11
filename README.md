# CrabOS 🦀

## Setup

You need a few things to build this:

* Rust nightly
* `cargo-xbuild`
* `nasm`
* A cross binutils targeting x86_64-elf

### MacOS

1. Install Rust with [rustup](https://rustup.rs/). Make sure install nightly and select it as your default compiler.

1. Install `nasm` from Homebrew:

    ```
    brew install nasm
    ```

1. Install `cargo-xbuild` from cargo:

    ```
    cargo install cargo-xbuild
    ```

1. Install binutils from source. I build my cross compilers into `~/cross`, but anywhere works.

    ```
    curl -O https://ftp.gnu.org/gnu/binutils/binutils-2.32.tar.gz
    tar xf binutils-2.32
    cd binutils-2.32
    ./configure --target=x86_64-elf --disable-werror --prefix=$HOME/cross
    make && make install
    ```

## Building

```
make
```

## Known Bugs

* QEMU's TCG accelerator (the default) has a buggy implementation of the FS.base and GS.base MSRs. Use the KVM or HVF accelerators instead.
