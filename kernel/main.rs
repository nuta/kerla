#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(const_panic)]
#![feature(custom_test_frameworks)]
#![feature(alloc_error_handler)]
#![feature(const_btree_new)]
#![feature(array_methods)]
#![test_runner(crate::test_runner::run_tests)]
#![reexport_test_harness_main = "test_main"]
#![allow(clippy::upper_case_acronyms)]
// FIXME:
#![allow(unaligned_references)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate log;

#[macro_use]
mod printk;
#[macro_use]
mod result;
#[macro_use]
mod arch;
#[macro_use]
mod user_buffer;
mod boot;
mod ctypes;
mod drivers;
mod fs;
mod interrupt;
mod lang_items;
mod mm;
mod net;
mod pipe;
mod poll;
mod prelude;
mod process;
mod random;
mod syscalls;
mod test_runner;
mod timer;
mod tty;
