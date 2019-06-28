fn main() {
    println!("cargo:rerun-if-changed=target/x86_64-kernel/start.o");
    println!("cargo:rerun-if-changed=target/x86_64-kernel/aux.o");
    println!("cargo:rerun-if-changed=target/x86_64-kernel/isrs.o");
}
