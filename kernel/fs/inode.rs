use crate::result::Result;

pub trait FileLike: Send + Sync {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize>;
}
