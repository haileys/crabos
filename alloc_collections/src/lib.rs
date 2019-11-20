#![no_std]
#![feature(allocator_api)]
#![feature(allow_internal_unstable)]
#![feature(arbitrary_self_types)]
// #![feature(box_patterns)]
#![feature(cfg_target_has_atomic)]
#![feature(coerce_unsized)]
#![feature(dispatch_from_dyn)]
#![feature(core_intrinsics)]
#![feature(dropck_eyepatch)]
#![feature(exact_size_is_empty)]
#![feature(fmt_internals)]
#![feature(fn_traits)]
#![feature(fundamental)]
#![feature(lang_items)]
#![feature(libc)]
#![feature(nll)]
#![feature(optin_builtin_traits)]
#![feature(pattern)]
#![feature(ptr_internals)]
#![feature(ptr_offset_from)]
#![feature(rustc_attrs)]
#![feature(receiver_trait)]
#![feature(slice_from_raw_parts)]
#![feature(specialization)]
// #![feature(staged_api)]
#![feature(std_internals)]
#![feature(str_internals)]
#![feature(trusted_len)]
#![feature(unboxed_closures)]
#![feature(unicode_internals)]
#![feature(unsize)]
#![feature(unsized_locals)]
#![feature(allocator_internals)]
#![feature(internal_uninit_const)]
#![feature(rustc_const_unstable)]
#![feature(slice_partition_dedup)]
#![feature(maybe_uninit_extra, maybe_uninit_slice)]
#![feature(alloc_layout_extra)]
#![feature(try_trait)]
#![feature(never_type)]

#[macro_use]
extern crate core;

pub mod boxed;
pub mod btree;
pub mod glue;

pub mod btree_map {
    //! A map based on a B-Tree.
    pub use super::btree::map::*;
}

// pub mod btree_set {
//     //! A set based on a B-Tree.
//     pub use super::btree::set::*;
// }
