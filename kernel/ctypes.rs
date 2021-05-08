//! C types.

#![allow(non_camel_case_types)]
use bitflags::bitflags;

pub type c_int16 = i16;
pub type c_int32 = i32;
pub type c_int64 = i64;
pub type c_uint32 = u32;
pub type c_uint64 = u64;

pub type c_int = c_int32;
pub type c_uint = c_uint32;
pub type c_short = c_int16;
pub type c_long = c_int64;
pub type c_ulong = c_uint64;

pub type c_time = c_int64;
pub type c_suseconds = c_int64;
pub type c_clockid = c_int;
pub type c_nfds = c_ulong;
pub type c_size = c_ulong;
pub type c_off = c_uint64;

pub const CLOCK_REALTIME: c_clockid = 0;
pub const CLOCK_MONOTONIC: c_clockid = 1;

bitflags! {
    pub struct MMapProt: c_int {
        const PROT_READ  = 1;
        const PROT_WRITE = 2;
        const PROT_EXEC  = 4;
    }
}

bitflags! {
    pub struct MMapFlags: c_int {
        const MAP_PRIVATE   = 0x02;
        const MAP_FIXED     = 0x10;
        const MAP_ANONYMOUS = 0x20;
        }
}
