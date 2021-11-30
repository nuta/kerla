#![no_std]

extern crate alloc;

mod block_dev;
mod layout;
pub mod super_block;
mod error;
mod tools;

/// default block size
pub const BLOCK_SIZE:usize = 1024;

pub use block_dev::BlockDevice;
pub use layout::Ext2SuperBlock;