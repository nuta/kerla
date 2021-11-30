pub type Result<T> = core::result::Result<T, FileSysError>;

/// Error definition
#[derive(Clone, Debug, PartialEq)]
pub enum FileSysError {
    Eof,
}
