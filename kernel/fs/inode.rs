use super::{opened_file::OpenOptions, path::PathBuf, stat::FileMode};
use crate::ctypes::c_short;
use crate::prelude::*;
use crate::{fs::stat::Stat, user_buffer::UserBufferMut};
use crate::{net::*, user_buffer::UserBuffer};
use bitflags::bitflags;
use kerla_utils::downcast::Downcastable;

/// The inode number.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct INodeNo(usize);

impl INodeNo {
    pub const fn new(no: usize) -> INodeNo {
        INodeNo(no)
    }

    pub const fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

bitflags! {
    pub struct PollStatus: c_short {
        const POLLIN     = 0x001;
        const POLLPRI    = 0x002;
        const POLLOUT    = 0x004;
        const POLLERR    = 0x008;
        const POLLHUP    = 0x010;
        const POLLNVAL   = 0x020;
        const POLLRDNORM = 0x040;
        const POLLRDBAND = 0x080;
        const POLLWRNORM = 0x100;
        const POLLWRBAND = 0x200;
    }
}

/// A file-like object.
///
/// This trait represents an object which behaves like a file such as files on
/// disks (aka. regular files), UDP/TCP sockets, device files like tty, etc.
///
/// # Locking
///
/// Some `FileLike` implementation assume **the owner process's lock are not held.**
///
/// The following code would cause a dead lock if `FileLike` implementation also
/// internally tries to lock the owner process:
///
/// ```ignore
/// // DON'T DO THIS: The owner process lock is held when `FileLike::stat()` is
/// // called. It will cause a dead lock if the method tries to lock it :/
/// current_process().opened_files().lock().get(fd)?.stat()?;
/// //                                     ^^^^^^^^^
/// //                               OpenedFileTable::get() returns &Arc<...>.
/// //                               The current process lock needs to be held.
///
/// // GOOD: The owner process is unlocked when `FileLike::stat()` is called :D
/// let opened_file: Arc<SpinLock<OpenedFile>> = current_process().get_opened_file_by_fd(fd)?;
/// opened_file.stat()?;
/// ```
pub trait FileLike: Send + Sync + Downcastable {
    /// `open(2)`.
    fn open(&self, _options: &OpenOptions) -> Result<Option<Arc<dyn FileLike>>> {
        Ok(None)
    }

    /// `stat(2)`.
    fn stat(&self) -> Result<Stat> {
        Err(Error::new(Errno::EBADF))
    }

    /// `readlink(2)`.
    fn readlink(&self) -> Result<PathBuf> {
        // "EINVAL - The named file is not a symbolic link." -- readlink(2)
        Err(Error::new(Errno::EINVAL))
    }

    /// `poll(2)` and `select(2)`.
    fn poll(&self) -> Result<PollStatus> {
        Err(Error::new(Errno::EBADF))
    }

    /// `ioctl(2)`.
    fn ioctl(&self, _cmd: usize, _arg: usize) -> Result<isize> {
        Err(Error::new(Errno::EBADF))
    }

    /// `read(2)`.
    fn read(
        &self,
        _offset: usize,
        _buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    /// `write(2)`.
    fn write(&self, _offset: usize, _buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    /// `bind(2)`.
    fn bind(&self, _sockaddr: SockAddr) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    /// `listen(2)`.
    fn listen(&self, _backlog: i32) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    /// `getsockname(2)`.
    fn getsockname(&self) -> Result<SockAddr> {
        Err(Error::new(Errno::EBADF))
    }

    /// `getpeername(2)`.
    fn getpeername(&self) -> Result<SockAddr> {
        Err(Error::new(Errno::EBADF))
    }

    /// `fsync(2)`.
    fn fsync(&self) -> Result<()> {
        Ok(())
    }

    /// `accept(2)`.
    fn accept(&self, _options: &OpenOptions) -> Result<(Arc<dyn FileLike>, SockAddr)> {
        Err(Error::new(Errno::EBADF))
    }

    /// `connect(2)`.
    fn connect(&self, _sockaddr: SockAddr, _options: &OpenOptions) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }

    /// `sendto(2)`.
    fn sendto(
        &self,
        _buf: UserBuffer<'_>,
        _sockaddr: Option<SockAddr>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        Err(Error::new(Errno::EBADF))
    }

    /// `recvfrom(2)`.
    fn recvfrom(
        &self,
        _buf: UserBufferMut<'_>,
        _flags: RecvFromFlags,
        _options: &OpenOptions,
    ) -> Result<(usize, SockAddr)> {
        Err(Error::new(Errno::EBADF))
    }

    fn chmod(&self, _mode: FileMode) -> Result<()> {
        Err(Error::new(Errno::EBADF))
    }
}

/// Represents `d_type` in `linux_dirent`. See `getdents64(2)` manual.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
#[non_exhaustive]
pub enum FileType {
    Directory = 4,
    Regular = 8,
    Link = 10,
}

/// A directory entry (ones returned from `readdir(3)`).
///
/// # Locking
///
/// See [`FileLike`] documentation.
pub struct DirEntry {
    pub inode_no: INodeNo,
    pub file_type: FileType,
    pub name: String,
}

/// Represents a directory.
pub trait Directory: Send + Sync + Downcastable {
    /// Looks for an existing file.
    fn lookup(&self, name: &str) -> Result<INode>;
    /// Creates a file. Returns `EEXIST` if the it already exists.
    fn create_file(&self, _name: &str, _mode: FileMode) -> Result<INode>;
    /// Creates a directory. Returns `EEXIST` if the it already exists.
    fn create_dir(&self, _name: &str, _mode: FileMode) -> Result<INode>;
    /// `stat(2)`.
    fn stat(&self) -> Result<Stat>;
    /// `readdir(2)`.
    fn readdir(&self, index: usize) -> Result<Option<DirEntry>>;
    /// `link(2)`.
    fn link(&self, _name: &str, _link_to: &INode) -> Result<()>;
    /// `fsync(2)`.
    fn fsync(&self) -> Result<()> {
        Ok(())
    }
    /// `readlink(2)`.
    fn readlink(&self) -> Result<PathBuf> {
        // "EINVAL - The named file is not a symbolic link." -- readlink(2)
        Err(Error::new(Errno::EINVAL))
    }
    /// `chmod`
    fn chmod(&mut self, mode: FileMode) -> Result<()>;
}

/// A symbolic link.
///
/// # Locking
///
/// See [`FileLike`] documentation.
pub trait Symlink: Send + Sync + Downcastable {
    /// `stat(2)`.
    fn stat(&self) -> Result<Stat>;
    /// The path linked to.
    fn linked_to(&self) -> Result<PathBuf>;
    /// `fsync(2)`.
    fn fsync(&self) -> Result<()> {
        Ok(())
    }
    fn chmod(&mut self, mode: FileMode) -> Result<()>;
}

/// An inode object.
///
/// # Locking
///
/// See [`FileLike`] documentation.
///
/// # See Also
///
/// - [`crate::fs::opened_file::OpenedFile`]
#[derive(Clone)]
pub enum INode {
    FileLike(Arc<dyn FileLike>),
    Directory(Arc<dyn Directory>),
    Symlink(Arc<dyn Symlink>),
}

impl INode {
    /// Unwraps as a file. If it's not, returns `Errno::EBADF`.
    pub fn as_file(&self) -> Result<&Arc<dyn FileLike>> {
        match self {
            INode::FileLike(file) => Ok(file),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    /// Unwraps as a directory. If it's not, returns `Errno::EBADF`.
    pub fn as_dir(&self) -> Result<&Arc<dyn Directory>> {
        match self {
            INode::Directory(dir) => Ok(dir),
            _ => Err(Error::new(Errno::EBADF)),
        }
    }

    /// Returns `true` if it's a file.
    pub fn is_file(&self) -> bool {
        matches!(self, INode::FileLike(_))
    }

    /// Returns `true` if it's a directory.
    pub fn is_dir(&self) -> bool {
        matches!(self, INode::Directory(_))
    }

    /// `stat(2)`.
    pub fn stat(&self) -> Result<Stat> {
        match self {
            INode::FileLike(file) => file.stat(),
            INode::Symlink(file) => file.stat(),
            INode::Directory(dir) => dir.stat(),
        }
    }

    /// `fsync(2)`.
    pub fn fsync(&self) -> Result<()> {
        match self {
            INode::FileLike(file) => file.fsync(),
            INode::Symlink(file) => file.fsync(),
            INode::Directory(dir) => dir.fsync(),
        }
    }

    /// `readlink(2)`.
    pub fn readlink(&self) -> Result<PathBuf> {
        match self {
            INode::FileLike(file) => file.readlink(),
            INode::Symlink(file) => file.linked_to(),
            INode::Directory(dir) => dir.readlink(),
        }
    }

    /// `chmod(2)`
    pub fn chmod(&mut self, mode: FileMode) -> Result<()> {
        match self {
            INode::FileLike(file) => file.chmod(mode),
            INode::Symlink(file) => file.chmod(mode),
            INode::Directory(dir) => dir.chmod(mode),
        }
    }
}

impl From<Arc<dyn FileLike>> for INode {
    fn from(file: Arc<dyn FileLike>) -> Self {
        INode::FileLike(file)
    }
}

impl From<Arc<dyn Directory>> for INode {
    fn from(dir: Arc<dyn Directory>) -> Self {
        INode::Directory(dir)
    }
}

impl From<Arc<dyn Symlink>> for INode {
    fn from(symlink: Arc<dyn Symlink>) -> Self {
        INode::Symlink(symlink)
    }
}
