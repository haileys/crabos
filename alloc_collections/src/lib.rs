#![no_std]
#![feature(allocator_api)]
#![feature(allow_internal_unstable)]
#![feature(arbitrary_self_types)]
#![feature(box_into_raw_non_null)]
#![feature(box_patterns)]
#![feature(box_syntax)]
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
#![feature(staged_api)]
#![feature(std_internals)]
#![feature(str_internals)]
#![feature(trusted_len)]
#![feature(try_reserve)]
#![feature(unboxed_closures)]
#![feature(unicode_internals)]
#![feature(unsize)]
#![feature(unsized_locals)]
#![feature(allocator_internals)]
#![feature(on_unimplemented)]
#![feature(rustc_const_unstable)]
#![feature(const_vec_new)]
#![feature(slice_partition_dedup)]
#![feature(maybe_uninit_extra, maybe_uninit_slice, maybe_uninit_array)]
#![feature(alloc_layout_extra)]
#![feature(try_trait)]
#![feature(mem_take)]

#[macro_use]
extern crate core;

pub mod btree;

pub mod btree_map {
    //! A map based on a B-Tree.
    pub use super::btree::map::*;
}

pub mod btree_set {
    //! A set based on a B-Tree.
    pub use super::btree::set::*;
}
