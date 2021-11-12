#![cfg_attr(feature = "no_std", no_std)]
#![feature(slice_internals)]
#![feature(const_maybe_uninit_assume_init)]
#![allow(unused)]

#[cfg(not(feature = "no_std"))]
#[macro_use]
extern crate std;

#[macro_use]
extern crate alloc;

pub mod ext2;