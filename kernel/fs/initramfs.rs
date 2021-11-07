//! Initramfs parser.
//! <https://www.kernel.org/doc/html/latest/driver-api/early-userspace/buffer-format.html>
use crate::{
    fs::{
        file_system::FileSystem,
        inode::{DirEntry, Directory, FileLike, FileType, INode, INodeNo},
        path::Path,
        stat::FileMode,
        stat::{Stat, S_IFDIR},
    },
    prelude::*,
    user_buffer::{UserBufWriter, UserBuffer, UserBufferMut},
};
use core::{fmt, str::from_utf8_unchecked};
use hashbrown::HashMap;
use kerla_utils::byte_size::ByteSize;
use kerla_utils::bytes_parser::BytesParser;
use kerla_utils::once::Once;

use super::{inode::Symlink, opened_file::OpenOptions, path::PathBuf};

fn parse_str_field(bytes: &[u8]) -> &str {
    unsafe { from_utf8_unchecked(bytes) }
}

fn parse_hex_field(bytes: &[u8]) -> usize {
    usize::from_str_radix(parse_str_field(bytes), 16).unwrap()
}

pub static INITRAM_FS: Once<Arc<InitramFs>> = Once::new();

struct InitramFsFile {
    filename: &'static str,
    data: &'static [u8],
    stat: Stat,
}

impl FileLike for InitramFsFile {
    fn read(&self, offset: usize, buf: UserBufferMut<'_>, _options: &OpenOptions) -> Result<usize> {
        if offset > self.data.len() {
            return Ok(0);
        }

        let mut writer = UserBufWriter::from(buf);
        writer.write_bytes(&self.data[offset..])
    }

    fn write(&self, _offset: usize, _buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        Err(Error::new(Errno::EROFS))
    }

    fn stat(&self) -> Result<Stat> {
        Ok(self.stat)
    }
}

impl fmt::Debug for InitramFsFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InitramFsFile")
            .field("name", &self.filename)
            .finish()
    }
}

enum InitramFsINode {
    File(Arc<InitramFsFile>),
    Directory(Arc<InitramFsDir>),
    Symlink(Arc<InitramFsSymlink>),
}
struct InitramFsDir {
    filename: &'static str,
    stat: Stat,
    files: HashMap<&'static str, InitramFsINode>,
}

impl Directory for InitramFsDir {
    fn stat(&self) -> Result<Stat> {
        Ok(self.stat)
    }

    fn link(&self, _name: &str, _link_to: &INode) -> Result<()> {
        Err(Error::new(Errno::ENOSYS))
    }

    fn readdir(&self, index: usize) -> Result<Option<DirEntry>> {
        let entry = self.files.values().nth(index).map(|entry| match entry {
            InitramFsINode::Directory(dir) => DirEntry {
                inode_no: dir.stat.inode_no,
                file_type: FileType::Directory,
                name: dir.filename.to_string(),
            },
            InitramFsINode::File(file) => DirEntry {
                inode_no: file.stat.inode_no,
                file_type: FileType::Regular,
                name: file.filename.to_string(),
            },
            InitramFsINode::Symlink(symlink) => DirEntry {
                inode_no: symlink.stat.inode_no,
                file_type: FileType::Link,
                name: symlink.filename.to_string(),
            },
        });

        Ok(entry)
    }

    fn lookup(&self, name: &str) -> Result<INode> {
        let initramfs_inode = self
            .files
            .get(name)
            .ok_or_else(|| Error::new(Errno::ENOENT))?;
        Ok(match initramfs_inode {
            InitramFsINode::File(file) => INode::FileLike(file.clone() as Arc<dyn FileLike>),
            InitramFsINode::Directory(dir) => INode::Directory(dir.clone() as Arc<dyn Directory>),
            InitramFsINode::Symlink(path) => INode::Symlink(path.clone()),
        })
    }

    fn create_file(&self, _name: &str, _mode: FileMode) -> Result<INode> {
        Err(Errno::EROFS.into())
    }

    fn create_dir(&self, _name: &str, _mode: FileMode) -> Result<INode> {
        Err(Errno::EROFS.into())
    }
}

impl fmt::Debug for InitramFsDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InitramFsDir")
            .field("name", &self.filename)
            .finish()
    }
}

struct InitramFsSymlink {
    filename: &'static str,
    stat: Stat,
    dst: PathBuf,
}

impl Symlink for InitramFsSymlink {
    fn stat(&self) -> Result<Stat> {
        Ok(self.stat)
    }

    fn linked_to(&self) -> Result<PathBuf> {
        Ok(self.dst.clone())
    }
}

impl fmt::Debug for InitramFsSymlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InitramFsSymlink")
            .field("name", &self.filename)
            .finish()
    }
}

pub struct InitramFs {
    root_dir: Arc<InitramFsDir>,
}

impl InitramFs {
    pub fn new(fs_image: &'static [u8]) -> InitramFs {
        let mut image = BytesParser::new(fs_image);
        let mut root_files = HashMap::new();
        let mut num_files = 0;
        let mut loaded_size = 0;
        loop {
            let magic = parse_hex_field(image.consume_bytes(6).unwrap());
            if magic != 0x070701 {
                panic!(
                    "initramfs: invalid magic (expected {:x}, got {:x})",
                    0x070701, magic
                );
            }

            let ino = parse_hex_field(image.consume_bytes(8).unwrap());
            let mode = FileMode::new(parse_hex_field(image.consume_bytes(8).unwrap()) as u32);
            let _uid = parse_hex_field(image.consume_bytes(8).unwrap());
            let _gid = parse_hex_field(image.consume_bytes(8).unwrap());
            let _nlink = parse_hex_field(image.consume_bytes(8).unwrap());
            let _mtime = parse_hex_field(image.consume_bytes(8).unwrap());
            let filesize = parse_hex_field(image.consume_bytes(8).unwrap());
            let _dev_major = parse_hex_field(image.consume_bytes(8).unwrap());
            let _dev_minor = parse_hex_field(image.consume_bytes(8).unwrap());

            // Skip c_rmaj and c_rmin.
            image.skip(16).unwrap();

            let path_len = parse_hex_field(image.consume_bytes(8).unwrap());
            assert!(path_len > 0);

            // Skip checksum.
            image.skip(8).unwrap();

            let mut path = parse_str_field(image.consume_bytes(path_len - 1).unwrap());
            if path.starts_with("./") {
                path = &path[1..];
            }
            if path == "TRAILER!!!" {
                break;
            }

            assert!(!path.is_empty());
            trace!("initramfs: \"{}\" ({})", path, ByteSize::new(filesize));

            // Skip the trailing '\0'.
            image.skip(1).unwrap();
            image.skip_until_alignment(4).unwrap();

            // Look for the parent directory for the file.
            let mut files = &mut root_files;
            let mut filename = None;
            let mut components = Path::new(path).components().peekable();
            while let Some(comp) = components.next() {
                if components.peek().is_none() {
                    filename = Some(comp);
                    break;
                }

                match files.get_mut(comp) {
                    Some(InitramFsINode::Directory(dir)) => {
                        files = &mut Arc::get_mut(dir).unwrap().files;
                    }
                    Some(_) => {
                        panic!(
                            "initramfs: invalid path '{}' ('{}' is not a directory)",
                            path, comp
                        );
                    }
                    None => {
                        panic!(
                            "initramfs: invalid path '{}' ('{}' does not exist)",
                            path, comp
                        );
                    }
                }
            }

            // Create a file or a directory under its parent.
            let data = image.consume_bytes(filesize).unwrap();
            if mode.is_symbolic_link() {
                let filename = filename.unwrap();
                files.insert(
                    filename,
                    InitramFsINode::Symlink(Arc::new(InitramFsSymlink {
                        filename,
                        stat: Stat {
                            inode_no: INodeNo::new(ino),
                            mode,
                            ..Stat::zeroed()
                        },
                        dst: PathBuf::from(core::str::from_utf8(data).unwrap()),
                    })),
                );
            } else if mode.is_directory() {
                let filename = filename.unwrap();
                files.insert(
                    filename,
                    InitramFsINode::Directory(Arc::new(InitramFsDir {
                        filename,
                        files: HashMap::new(),
                        stat: Stat {
                            inode_no: INodeNo::new(ino),
                            mode,
                            ..Stat::zeroed()
                        },
                    })),
                );
            } else if mode.is_regular_file() {
                let filename = filename.unwrap();
                files.insert(
                    filename,
                    InitramFsINode::File(Arc::new(InitramFsFile {
                        filename,
                        data,
                        stat: Stat {
                            inode_no: INodeNo::new(ino),
                            mode,
                            ..Stat::zeroed()
                        },
                    })),
                );
            }

            image.skip_until_alignment(4).unwrap();
            num_files += 1;
            loaded_size += data.len();
        }

        info!(
            "initramfs: loaded {} files and directories ({})",
            num_files,
            ByteSize::new(loaded_size)
        );

        InitramFs {
            root_dir: Arc::new(InitramFsDir {
                // TODO: Should we use other value for the root directory?
                filename: "",
                stat: Stat {
                    inode_no: INodeNo::new(2),
                    mode: FileMode::new(S_IFDIR | 0o755),
                    ..Stat::zeroed()
                },
                files: root_files,
            }),
        }
    }
}

impl FileSystem for InitramFs {
    fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        Ok(self.root_dir.clone())
    }
}

pub fn init() {
    INITRAM_FS.init(|| {
        let image = include_bytes!(concat!("../../", env!("INITRAMFS_PATH")));
        Arc::new(InitramFs::new(image))
    });
}
