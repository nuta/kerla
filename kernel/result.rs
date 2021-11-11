use core::fmt;

use kerla_runtime::{
    address::{AccessError, NullUserPointerError},
    page_allocator::PageAllocError,
};

#[cfg(debug_assertions)]
use kerla_runtime::backtrace::CapturedBacktrace;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
#[allow(unused)]
#[allow(clippy::upper_case_acronyms)]
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

    EADDRINUSE = 98,
    EADDRNOTAVAIL = 99,
    ENETDOWN = 100,
    ENETUNREACH = 101,
    ENETRESET = 102,
    ECONNABORTED = 103,
    ECONNRESET = 104,
    ENOBUFS = 105,
    EISCONN = 106,
    ENOTCONN = 107,
}

pub type Result<T> = ::core::result::Result<T, Error>;

enum ErrorMessage {
    StaticStr(&'static str),
}

pub struct Error {
    errno: Errno,
    message: Option<ErrorMessage>,
    #[cfg(debug_assertions)]
    backtrace: Option<CapturedBacktrace>,
}

impl Error {
    pub fn new(errno: Errno) -> Error {
        Error {
            errno,
            message: None,
            #[cfg(debug_assertions)]
            backtrace: Some(CapturedBacktrace::capture()),
        }
    }

    pub fn with_message(errno: Errno, message: &'static str) -> Error {
        Error {
            errno,
            message: Some(ErrorMessage::StaticStr(message)),
            #[cfg(debug_assertions)]
            backtrace: Some(CapturedBacktrace::capture()),
        }
    }

    pub const fn with_message_const(errno: Errno, message: &'static str) -> Error {
        Error {
            errno,
            message: Some(ErrorMessage::StaticStr(message)),
            #[cfg(debug_assertions)]
            backtrace: None,
        }
    }

    pub fn errno(&self) -> Errno {
        self.errno
    }
}

impl fmt::Debug for Error {
    #[cfg(not(debug_assertions))]
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

    #[cfg(debug_assertions)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = self.message.as_ref() {
            match message {
                ErrorMessage::StaticStr(message) => {
                    if let Some(ref trace) = self.backtrace {
                        write!(
                            f,
                            "[{:?}] {}\n    This error originates from:\n{:?}",
                            self.errno, message, trace
                        )
                    } else {
                        write!(f, "[{:?}] {}", self.errno, message)
                    }
                }
            }
        } else if let Some(ref trace) = self.backtrace {
            write!(
                f,
                "{:?}: This error originates from:\n{:?}",
                self.errno, trace
            )
        } else {
            write!(f, "{:?}", self.errno)
        }
    }
}

impl From<Errno> for Error {
    fn from(errno: Errno) -> Error {
        Error::new(errno)
    }
}

impl From<AccessError> for Error {
    fn from(_error: AccessError) -> Error {
        Error::new(Errno::EFAULT)
    }
}

impl From<PageAllocError> for Error {
    fn from(_error: PageAllocError) -> Error {
        Error::new(Errno::ENOMEM)
    }
}

impl From<NullUserPointerError> for Error {
    fn from(_error: NullUserPointerError) -> Error {
        Error::new(Errno::EFAULT)
    }
}

impl From<smoltcp::Error> for Error {
    fn from(error: smoltcp::Error) -> Error {
        match error {
            smoltcp::Error::Exhausted => Error::with_message(Errno::EINVAL, "smoltcp(Exhausted)"),
            smoltcp::Error::Illegal => Error::with_message(Errno::EINVAL, "smoltcp(Illegal)"),
            smoltcp::Error::Unaddressable => {
                Error::with_message(Errno::EINVAL, "smoltcp(Unaddressable)")
            }
            smoltcp::Error::Finished => Error::with_message(Errno::EINVAL, "smoltcp(Finished)"),
            smoltcp::Error::Truncated => Error::with_message(Errno::EINVAL, "smoltcp(Truncated)"),
            smoltcp::Error::Checksum => Error::with_message(Errno::EINVAL, "smoltcp(Checksum)"),
            smoltcp::Error::Unrecognized => {
                Error::with_message(Errno::EINVAL, "smoltcp(Unrecognized)")
            }
            smoltcp::Error::Fragmented => Error::with_message(Errno::EINVAL, "smoltcp(Fragmented)"),
            smoltcp::Error::Malformed => Error::with_message(Errno::EINVAL, "smoltcp(Malformed)"),
            smoltcp::Error::Dropped => Error::with_message(Errno::ENOMEM, "smoltcp(Dropped)"),
            _ => unreachable!(),
        }
    }
}
