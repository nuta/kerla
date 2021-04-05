#![allow(non_camel_case_types)]

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
pub type c_clockid = c_int;
pub type c_nfds = c_ulong;

pub const CLOCK_REALTIME: c_clockid = 0;
