use core::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
pub enum Errno {
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    E2BIG = 7,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    ENOTBLK = 15,
    EBUSY = 16,
    EEXIST = 17,
    EXDEV = 18,
    ENODEV = 19,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    ENFILE = 23,
    EMFILE = 24,
    ENOTTY = 25,
    ETXTBSY = 26,
    EFBIG = 27,
    ENOSPC = 28,
    ESPIPE = 29,
    EROFS = 30,
    EMLINK = 31,
    EPIPE = 32,
    EDOM = 33,
    ERANGE = 34,

    ENOSYS = 38,
    ELOOP = 40,
}

pub type Result<T> = ::core::result::Result<T, Error>;

enum ErrorMessage {
    StaticStr(&'static str),
}

pub struct Error {
    errno: Errno,
    message: Option<ErrorMessage>,
}

impl Error {
    pub fn new(errno: Errno) -> Error {
        Error {
            errno,
            message: None,
        }
    }

    pub const fn with_message(errno: Errno, message: &'static str) -> Error {
        Error {
            errno,
            message: Some(ErrorMessage::StaticStr(message)),
        }
    }

    pub fn errno(&self) -> Errno {
        self.errno
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = self.message.as_ref() {
            match message {
                ErrorMessage::StaticStr(message) => {
                    write!(f, "[{:?}] {}", self.errno, message)
                }
            }
        } else {
            write!(f, "{:?}", self.errno)
        }
    }
}

pub trait ErrorExt<T> {
    fn into_error(self, errno: Errno) -> Result<T>;
    fn into_error_with_message(self, errno: Errno, message: &'static str) -> Result<T>;
}

impl<T> ErrorExt<T> for Option<T> {
    fn into_error(self, errno: Errno) -> Result<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(Error::new(errno)),
        }
    }

    fn into_error_with_message(self, errno: Errno, message: &'static str) -> Result<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(Error::with_message(errno, message)),
        }
    }
}
