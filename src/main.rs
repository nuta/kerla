#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner::run_tests)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
mod printk;
mod arch;
mod boot;
mod lang_items;
mod test_runner;
