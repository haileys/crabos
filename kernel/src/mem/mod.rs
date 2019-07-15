pub mod fault;
pub mod kalloc;
pub mod kvirt;
pub mod page;
pub mod phys;
pub mod user;

#[derive(Debug)]
pub struct MemoryExhausted;
