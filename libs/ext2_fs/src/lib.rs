#![no_std]

extern crate alloc;

mod block_dev;
mod error;
mod layout;
mod super_block;

pub const BLOCK_SIZE:usize = 1024;

pub use block_dev::BlockDevice;
pub use layout::*;
pub use super_block::*;
