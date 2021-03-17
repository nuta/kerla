#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(const_panic)]
#![feature(custom_test_frameworks)]
#![feature(alloc_error_handler)]
#![feature(const_btree_new)]
#![test_runner(crate::test_runner::run_tests)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate alloc;

#[macro_use]
mod printk;
mod allocator;
mod arch;
mod boot;
mod lang_items;
mod test_runner;
mod utils;
