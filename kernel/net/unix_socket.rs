use alloc::sync::Arc;

use crate::{
    fs::{inode::FileLike, opened_file::OpenOptions},
    result::{Errno, Result},
};

use super::Endpoint;

pub struct UnixSocket {}

impl UnixSocket {
    pub fn new() -> Arc<UnixSocket> {
        Arc::new(UnixSocket {})
    }
}

impl FileLike for UnixSocket {
    fn connect(&self, _endpoint: Endpoint, _options: &OpenOptions) -> Result<()> {
        Err(Errno::EACCES.into())
    }
}
