#![cfg_attr(feature = "no_std", no_std)]
#![feature(slice_internals)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(const_fn_trait_bound)]
#![allow(unused)]

#[cfg(not(feature = "no_std"))]
#[macro_use]
extern crate std;

#[macro_use]
extern crate alloc;


pub mod ext2;
pub mod disk;
pub mod error;

pub mod super_block;
mod test;

#[warn(non_camel_case_types)]
mod endian_tool;


/// it should be in kernel
pub struct SuperBlock {
    //TODO
}