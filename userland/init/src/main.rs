#![no_std]
#![no_main]
#![feature(asm)]
#![feature(core_panic)]

use crabapi::fs::File;
use crabapi::io::{self, Read, Write};
use crabapi::task;

#[no_mangle]
pub extern "C" fn main() {
    let mut con = io::console();

    let mut buf = [0u8; 32];
    con.read(&mut buf)
        .expect("Console::read");

    con.write_all(b"\n\nWelcome.\n\n")
        .expect("Console::write_all");

    let mut init = File::open(b"/init.bin")
        .expect("File::open");

    let mut buf = [0u8; 128];
    init.read(&mut buf)
        .expect("File::read");

    con.write_all(&buf)
        .expect("Console::write_all");

    task::exit(0);
}
