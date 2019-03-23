fn main() {
    println!("cargo:rerun-if-changed=target/x86_64-kernel/start.o");
}
