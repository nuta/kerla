#![allow(non_camel_case_types)]

pub type c_int = i32;
pub type c_uint = u32;

pub type c_int64 = i64;
pub type c_long = c_int64;
pub type c_time = c_int64;
pub type c_clockid = c_int;

pub const CLOCK_REALTIME: c_clockid = 0;
