#![no_std]
#![feature(asm)]
#![feature(global_asm)]

extern crate alloc;

#[macro_use]
extern crate log;

#[macro_use]
pub mod printk;

pub mod addr;
pub mod backtrace;
pub mod bootinfo;
pub mod global_allocator;
pub mod page_allocator;
pub mod result;
pub mod spinlock;

mod arch;
