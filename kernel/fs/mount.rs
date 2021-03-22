use super::{
    file_system::FileSystem,
    inode::{Directory, FileLike, INode, INodeNo},
    path::Path,
};
use crate::result::{Errno, Error, Result};
use alloc::sync::Arc;
use alloc::vec::Vec;
use hashbrown::HashMap;

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
        Ok(self.mount_points.get(&dir.stat()?.inode_no))
    }

    pub fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        self.root.fs.root_dir()
    }

    /// Resolves a path into an file.
    pub fn lookup_file(&self, path: &str) -> Result<Arc<dyn FileLike>> {
        match self.lookup_inode(self.root_dir()?, Path::new(path))? {
            INode::Directory(_) => Err(Error::new(Errno::EISDIR)),
            INode::FileLike(file) => Ok(file),
        }
    }

    /// Resolves a path into an directory.
    pub fn lookup_dir(&self, path: &str) -> Result<Arc<dyn Directory>> {
        match self.lookup_inode(self.root_dir()?, Path::new(path))? {
            INode::Directory(dir) => Ok(dir),
            INode::FileLike(_) => Err(Error::new(Errno::EISDIR)),
        }
    }

    /// Resolves a path into an inode.
    pub fn lookup_inode<'a>(
        &self,
        lookup_from: Arc<dyn Directory>,
        path: Path<'a>,
    ) -> Result<INode> {
        let mut current_dir = lookup_from;
        let mut components = path.components().peekable();
        while let Some(name) = components.next() {
            let entry = current_dir.lookup(name)?;
            match (components.peek(), entry.inode) {
                // Found the matching file.
                (None, inode) => return Ok(inode),
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
