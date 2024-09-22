use super::{
    file_system::FileSystem,
    inode::{Directory, FileLike, INode, INodeNo},
    opened_file::OpenedFileTable,
    opened_file::PathComponent,
    path::Path,
};
use crate::prelude::*;
use crate::syscalls::CwdOrFd;

use hashbrown::HashMap;

const DEFAULT_SYMLINK_FOLLOW_MAX: usize = 8;

pub struct MountPoint {
    fs: Arc<dyn FileSystem>,
}

pub struct RootFs {
    root_path: Arc<PathComponent>,
    cwd_path: Arc<PathComponent>,
    mount_points: HashMap<INodeNo, MountPoint>,
    symlink_follow_limit: usize,
}

impl RootFs {
    pub fn new(root: Arc<dyn FileSystem>) -> Result<RootFs> {
        let root_path = Arc::new(PathComponent {
            parent_dir: None,
            name: String::new(),
            inode: root.root_dir()?.into(),
        });

        Ok(RootFs {
            mount_points: HashMap::new(),
            root_path: root_path.clone(),
            cwd_path: root_path,
            symlink_follow_limit: DEFAULT_SYMLINK_FOLLOW_MAX,
        })
    }

    pub fn mount(&mut self, dir: Arc<dyn Directory>, fs: Arc<dyn FileSystem>) -> Result<()> {
        self.mount_points
            .insert(dir.stat()?.inode_no, MountPoint { fs });
        Ok(())
    }

    /// Resolves a path (from the current working directory) into an inode.
    /// This method resolves symbolic links: it will never return `INode::Symlink`.
    pub fn lookup(&self, path: &Path) -> Result<INode> {
        self.lookup_inode(path, true)
    }

    /// Resolves a path (from the current working directory) into an inode without
    /// following symlinks.
    pub fn lookup_no_symlink_follow(&self, path: &Path) -> Result<INode> {
        self.lookup_inode(path, false)
    }

    /// Resolves a path (from the current working directory) into an file.
    pub fn lookup_file(&self, path: &Path) -> Result<Arc<dyn FileLike>> {
        match self.lookup(path)? {
            INode::Directory(_) => Err(Error::new(Errno::EISDIR)),
            INode::FileLike(file) => Ok(file),
            // Symbolic links should be already resolved.
            INode::Symlink(_) => unreachable!(),
        }
    }

    /// Resolves a path (from the current working directory) into an directory.
    pub fn lookup_dir(&self, path: &Path) -> Result<Arc<dyn Directory>> {
        match self.lookup(path)? {
            INode::Directory(dir) => Ok(dir),
            INode::FileLike(_) => Err(Error::new(Errno::EISDIR)),
            // Symbolic links should be already resolved.
            INode::Symlink(_) => unreachable!(),
        }
    }

    /// Changes the current working directory.
    pub fn chdir(&mut self, path: &Path) -> Result<()> {
        self.cwd_path = self.lookup_path(path, true)?;
        Ok(())
    }

    pub fn cwd_path(&self) -> &PathComponent {
        &self.cwd_path
    }

    /// Resolves a path into an inode. If `follow_symlink` is `true`, symbolic
    /// linked are resolved and will never return `INode::Symlink`.
    pub fn lookup_inode(&self, path: &Path, follow_symlink: bool) -> Result<INode> {
        self.lookup_path(path, follow_symlink)
            .map(|path_comp| path_comp.inode.clone())
    }

    fn lookup_mount_point(&self, dir: &Arc<dyn Directory>) -> Result<Option<&MountPoint>> {
        let stat = dir.stat()?;
        let inode_no = stat.inode_no; // Move out of unaligned
        Ok(self.mount_points.get(&inode_no))
    }

    /// Resolves a path into `PathComponent`. If `follow_symlink` is `true`,
    /// symbolic links are resolved and will never return `INode::Symlink`.
    pub fn lookup_path(&self, path: &Path, follow_symlink: bool) -> Result<Arc<PathComponent>> {
        let lookup_from = if path.is_absolute() {
            self.root_path.clone()
        } else {
            self.cwd_path.clone()
        };

        self.do_lookup_path(
            &lookup_from,
            path,
            follow_symlink,
            self.symlink_follow_limit,
        )
    }

    /// Resolves a path into `PathComponent` from the given directory `cwd_or_fd`.
    /// If `follow_symlink` is `true`, symbolic links are resolved and will
    /// never return `INode::Symlink`.
    pub fn lookup_path_at(
        &self,
        opened_files: &OpenedFileTable,
        cwd_or_fd: &CwdOrFd,
        path: &Path,
        follow_symlink: bool,
    ) -> Result<Arc<PathComponent>> {
        self.do_lookup_path(
            &self.resolve_cwd_or_fd(opened_files, cwd_or_fd, path)?,
            path,
            follow_symlink,
            self.symlink_follow_limit,
        )
    }

    pub fn lookup_parent_path_at<'a>(
        &self,
        opened_files: &OpenedFileTable,
        cwd_or_fd: &CwdOrFd,
        path: &'a Path,
        follow_symlink: bool,
    ) -> Result<(Arc<PathComponent>, &'a str)> {
        let (parent_dir, name) = path
            .parent_and_basename()
            .ok_or_else::<Error, _>(|| Errno::EEXIST.into())?;
        let path = self.lookup_path_at(opened_files, cwd_or_fd, parent_dir, follow_symlink)?;
        Ok((path, name))
    }

    fn resolve_cwd_or_fd(
        &self,
        opened_files: &OpenedFileTable,
        cwd_or_fd: &CwdOrFd,
        path: &Path,
    ) -> Result<Arc<PathComponent>> {
        if path.is_absolute() {
            Ok(self.root_path.clone())
        } else {
            match cwd_or_fd {
                CwdOrFd::AtCwd => Ok(self.cwd_path.clone()),
                CwdOrFd::Fd(fd) => {
                    let opened_file = opened_files.get(*fd)?;
                    Ok(opened_file.path().clone())
                }
            }
        }
    }

    fn do_lookup_path(
        &self,
        lookup_from: &Arc<PathComponent>,
        path: &Path,
        follow_symlink: bool,
        symlink_follow_limit: usize,
    ) -> Result<Arc<PathComponent>> {
        if path.is_empty() {
            return Err(Error::new(Errno::ENOENT));
        }

        let mut parent_dir = lookup_from.clone();

        // Iterate and resolve each component (e.g. `a`, `b`, and `c` in `a/b/c`).
        let mut components = path.components().peekable();
        while let Some(name) = components.next() {
            let path_comp = match name {
                // Handle some special cases that appear in a relative path.
                "." => continue,
                ".." => parent_dir
                    .parent_dir
                    .as_ref()
                    .unwrap_or(&self.root_path)
                    .clone(),
                // Look for the entry with the name in the directory.
                _ => {
                    let inode = match parent_dir.inode.as_dir()?.lookup(name)? {
                        // If it is a directory and it's a mount point, go
                        // into the mounted file system's root.
                        INode::Directory(dir) => match self.lookup_mount_point(&dir)? {
                            Some(mount_point) => mount_point.fs.root_dir()?.into(),
                            None => dir.into(),
                        },
                        inode => inode,
                    };

                    Arc::new(PathComponent {
                        parent_dir: Some(parent_dir.clone()),
                        name: name.to_owned(),
                        inode,
                    })
                }
            };

            if components.peek().is_some() {
                // Ancestor components: `a` and `b` in `a/b/c`. Visit the next
                // level directory.
                parent_dir = match &path_comp.inode {
                    INode::Directory(_) => path_comp,
                    INode::Symlink(symlink) => {
                        // Follow the symlink even if follow_symlinks is false since
                        // it's not the last one of the path components.

                        if symlink_follow_limit == 0 {
                            return Err(Errno::ELOOP.into());
                        }

                        let linked_to = symlink.linked_to()?;
                        let follow_from = if linked_to.is_absolute() {
                            &self.root_path
                        } else {
                            &parent_dir
                        };

                        let dst_path = self.do_lookup_path(
                            follow_from,
                            &linked_to,
                            follow_symlink,
                            symlink_follow_limit - 1,
                        )?;

                        // Check if the desitnation is a directory.
                        match &dst_path.inode {
                            INode::Directory(_) => dst_path,
                            _ => return Err(Errno::ENOTDIR.into()),
                        }
                    }
                    INode::FileLike(_) => {
                        // The next level must be an directory since the current component
                        // is not the last one.
                        return Err(Errno::ENOTDIR.into());
                    }
                }
            } else {
                // The last component: `c` in `a/b/c`.
                match &path_comp.inode {
                    INode::Symlink(symlink) if follow_symlink => {
                        if symlink_follow_limit == 0 {
                            return Err(Errno::ELOOP.into());
                        }

                        let linked_to = symlink.linked_to()?;
                        let follow_from = if linked_to.is_absolute() {
                            &self.root_path
                        } else {
                            &parent_dir
                        };

                        return self.do_lookup_path(
                            follow_from,
                            &linked_to,
                            follow_symlink,
                            symlink_follow_limit - 1,
                        );
                    }
                    _ => {
                        return Ok(path_comp);
                    }
                }
            }
        }

        // Here's reachable if the path points to the root (i.e. "/") or the path
        // ends with "." (e.g. "." and "a/b/c/.").
        Ok(parent_dir)
    }
}
