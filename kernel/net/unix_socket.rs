use core::fmt;

use alloc::sync::Arc;

use crate::{
    fs::{inode::FileLike, opened_file::OpenOptions},
    net::socket::SockAddr,
    result::{Errno, Result},
};

pub struct UnixSocket {}

impl UnixSocket {
    pub fn new() -> Arc<UnixSocket> {
        Arc::new(UnixSocket {})
    }
}

impl FileLike for UnixSocket {
    fn connect(&self, _endpoint: SockAddr, _options: &OpenOptions) -> Result<()> {
        Err(Errno::EACCES.into())
    }
}

impl fmt::Debug for UnixSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnixSocket").finish()
    }
}
