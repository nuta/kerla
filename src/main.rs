#![no_std]
#![no_main]
#![feature(asm)]
#![feature(global_asm)]

#[macro_use]
mod printk;

mod arch;
mod boot;
mod lang_items;
