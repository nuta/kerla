use crate::{
    fs::{inode::FileLike, opened_file::OpenOptions, stat::Stat},
    result::Result,
    user_buffer::UserBuffer,
    user_buffer::UserBufferMut,
};

/// The `/dev/null` file.
pub(super) struct NullFile {}
impl NullFile {
    pub fn new() -> NullFile {
        NullFile {}
    }
}

impl FileLike for NullFile {
    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }

    fn read(
        &self,
        _offset: usize,
        _buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        Ok(0)
    }

    fn write(&self, _offset: usize, buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        Ok(buf.remaining_len())
    }
}
