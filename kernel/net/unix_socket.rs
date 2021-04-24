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
