#[derive(Debug)]
pub enum Errno {}

pub type Result<T> = ::core::result::Result<T, Errno>;
