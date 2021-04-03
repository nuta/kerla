use super::{
    file_system::FileSystem,
    inode::{Directory, FileLike, INode, INodeNo},
    path::Path,
};
use crate::result::{Errno, Error, Result};
use alloc::sync::Arc;

use hashbrown::HashMap;

const DEFAULT_SYMLINK_FOLLOW_MAX: usize = 8;

pub struct MountPoint {
    fs: Arc<dyn FileSystem>,
}

pub struct RootFs {
    root: MountPoint,
    mount_points: HashMap<INodeNo, MountPoint>,
}

impl RootFs {
    pub fn new(root: Arc<dyn FileSystem>) -> RootFs {
        RootFs {
            root: MountPoint { fs: root },
            mount_points: HashMap::new(),
        }
    }

    pub fn mount(&mut self, dir: Arc<dyn Directory>, fs: Arc<dyn FileSystem>) -> Result<()> {
        self.mount_points
            .insert(dir.stat()?.inode_no, MountPoint { fs });
        Ok(())
    }

    pub fn lookup_mount_point(&self, dir: &Arc<dyn Directory>) -> Result<Option<&MountPoint>> {
        let stat = dir.stat()?;
        Ok(self.mount_points.get(unsafe { &stat.inode_no }))
    }

    pub fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        self.root.fs.root_dir()
    }

    /// Resolves a path into an inode.
    pub fn lookup(&self, path: &str) -> Result<INode> {
        self.lookup_inode(&self.root_dir()?, Path::new(path), true)
    }

    /// Resolves a path into an file.
    pub fn lookup_file(&self, path: &str) -> Result<Arc<dyn FileLike>> {
        match self.lookup(path)? {
            INode::Directory(_) => Err(Error::new(Errno::EISDIR)),
            INode::FileLike(file) => Ok(file),
            // Symbolic links should be already resolved.
            INode::Symlink(_) => unreachable!(),
        }
    }

    /// Resolves a path into an directory.
    pub fn lookup_dir(&self, path: &str) -> Result<Arc<dyn Directory>> {
        match self.lookup(path)? {
            INode::Directory(dir) => Ok(dir),
            INode::FileLike(_) => Err(Error::new(Errno::EISDIR)),
            // Symbolic links should be already resolved.
            INode::Symlink(_) => unreachable!(),
        }
    }

    /// Resolves a path into an inode. If `follow_symlink` is `true`, symbolic
    /// linked are resolved and will never return `INode::Symlink`.
    pub fn lookup_inode(
        &self,
        lookup_from: &Arc<dyn Directory>,
        path: &Path,
        follow_symlink: bool,
    ) -> Result<INode> {
        self.do_lookup_inode(
            lookup_from.clone(),
            path,
            follow_symlink,
            DEFAULT_SYMLINK_FOLLOW_MAX,
        )
    }

    fn do_lookup_inode(
        &self,
        lookup_from: Arc<dyn Directory>,
        path: &Path,
        follow_symlink: bool,
        symlink_follow_limit: usize,
    ) -> Result<INode> {
        if path == Path::new("/") {
            return Ok(INode::Directory(lookup_from));
        }

        let mut current_dir = lookup_from;
        let mut components = path.components().peekable();
        while let Some(name) = components.next() {
            match (components.peek(), current_dir.lookup(name)?) {
                // Found the matching file.
                (None, INode::Symlink(symlink)) if follow_symlink => {
                    if symlink_follow_limit == 0 {
                        return Err(Error::new(Errno::ELOOP));
                    }

                    let linked_to = symlink.linked_to()?;
                    let follow_from = if linked_to.is_absolute() {
                        self.root_dir()?
                    } else {
                        current_dir
                    };

                    return self.do_lookup_inode(
                        follow_from,
                        &linked_to,
                        follow_symlink,
                        symlink_follow_limit - 1,
                    );
                }
                (None, inode) => {
                    return Ok(inode);
                }
                (Some(_), INode::Directory(dir)) => match self.lookup_mount_point(&dir)? {
                    Some(mount_point) => {
                        // The next level directory is a mount point. Go into the root
                        // directory of the mounted file system.
                        current_dir = mount_point.fs.root_dir()?;
                    }
                    None => {
                        // Go into the next level directory.
                        current_dir = dir;
                    }
                },
                (Some(_), INode::Symlink(symlink)) => {
                    // Follow the symlink even if follow_symlinks is false since
                    // it's not the last one of the path components.

                    if symlink_follow_limit == 0 {
                        return Err(Error::new(Errno::ELOOP));
                    }

                    let linked_to = symlink.linked_to()?;
                    let follow_from = if linked_to.is_absolute() {
                        self.root_dir()?
                    } else {
                        current_dir
                    };

                    let linked_inode = self.do_lookup_inode(
                        follow_from,
                        &linked_to,
                        follow_symlink,
                        symlink_follow_limit - 1,
                    )?;

                    current_dir = match linked_inode {
                        INode::Directory(dir) => dir,
                        _ => return Err(Error::new(Errno::ENOTDIR)),
                    }
                }
                // The next level must be an directory since the current component
                // is not the last one.
                (Some(_), INode::FileLike(_)) => {
                    return Err(Error::new(Errno::ENOTDIR));
                }
            }
        }

        // Here is reachable if path is empty.
        Err(Error::new(Errno::ENOENT))
    }
}
