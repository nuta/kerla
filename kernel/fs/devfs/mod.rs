use self::{null::NullFile, tty::Tty};

use crate::{
    fs::{
        file_system::FileSystem,
        inode::{Directory, FileLike},
    },
    result::Result,
    tty::pty::Ptmx,
};
use alloc::sync::Arc;
use kerla_utils::once::Once;

use super::tmpfs::TmpFs;

mod null;
mod tty;

pub static DEV_FS: Once<Arc<DevFs>> = Once::new();
static NULL_FILE: Once<Arc<dyn FileLike>> = Once::new();
pub static SERIAL_TTY: Once<Arc<Tty>> = Once::new();
pub static PTMX: Once<Arc<Ptmx>> = Once::new();

pub struct DevFs(TmpFs);

impl DevFs {
    pub fn new() -> DevFs {
        let tmpfs = TmpFs::new();
        let root_dir = tmpfs.root_tmpfs_dir();
        let pts_dir = root_dir.add_dir("pts");

        NULL_FILE.init(|| Arc::new(NullFile::new()) as Arc<dyn FileLike>);
        SERIAL_TTY.init(|| Arc::new(Tty::new("serial")));
        PTMX.init(|| Arc::new(Ptmx::new(pts_dir)));

        root_dir.add_file("null", NULL_FILE.clone());
        root_dir.add_file("tty", SERIAL_TTY.clone() as Arc<dyn FileLike>);
        root_dir.add_file("console", SERIAL_TTY.clone() as Arc<dyn FileLike>);
        root_dir.add_file("ptmx", PTMX.clone() as Arc<dyn FileLike>);

        DevFs(tmpfs)
    }
}

impl FileSystem for DevFs {
    fn root_dir(&self) -> Result<Arc<dyn Directory>> {
        self.0.root_dir()
    }
}

pub fn init() {
    DEV_FS.init(|| Arc::new(DevFs::new()));
}
