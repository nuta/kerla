use crate::{
    arch::{self, disable_interrupt, enable_interrupt, is_interrupt_enabled, SpinLock, VAddr},
    elf::Elf,
    fs::initramfs::INITRAM_FS,
    fs::mount::RootFs,
    fs::opened_file,
    fs::path::Path,
    fs::{
        devfs::DEV_FS,
        inode::{FileLike, INode},
        opened_file::*,
        stat::Stat,
    },
    mm::{
        page_allocator::alloc_pages,
        vm::{Vm, VmAreaType},
    },
    result::{Errno, Error, ErrorExt, Result},
};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::{UserVAddr, KERNEL_STACK_SIZE, PAGE_SIZE, USER_STACK_TOP};
use arrayvec::ArrayVec;
use core::cmp::max;
use core::mem::{self, size_of, size_of_val};
use core::sync::atomic::{AtomicI32, Ordering};
use goblin::elf64::program_header::PT_LOAD;
use opened_file::OpenedFileTable;
use penguin_utils::once::Once;
use penguin_utils::{alignment::align_up, lazy::Lazy};

pub mod execve;
mod init_stack;
pub mod process;
pub mod scheduler;
pub mod switch;
pub mod wait_queue;

pub use execve::*;
pub use init_stack::*;
pub use process::*;
pub use scheduler::*;
pub use switch::*;
pub use wait_queue::*;

cpu_local! {
    static ref CURRENT: Lazy<Arc<Process>> = Lazy::new();
}

cpu_local! {
    // TODO: Should be pub(super)
    pub static ref IDLE_THREAD: Lazy<Arc<Process>> = Lazy::new();
}

static SCHEDULER: Once<SpinLock<Scheduler>> = Once::new();

pub fn current_process() -> &'static Arc<Process> {
    CURRENT.get()
}

pub fn init() {
    SCHEDULER.init(|| SpinLock::new(Scheduler::new()));
    let idle_thread = Process::new_idle_thread().unwrap();
    IDLE_THREAD.as_mut().set(idle_thread.clone());
    CURRENT.as_mut().set(idle_thread);

    let mut root_fs = RootFs::new(INITRAM_FS.clone());
    let root_dir = root_fs.root_dir().expect("failed to open the root dir");
    root_fs
        .mount(
            root_fs.lookup_dir("/dev").expect("failed to locate /dev"),
            DEV_FS.clone(),
        )
        .expect("failed to mount devfs");

    let console = root_fs
        .lookup_inode(&root_dir, Path::new("/dev/console"), true)
        .expect("failed to open /dev/console");

    let inode = root_fs
        .lookup_inode(&root_dir, Path::new("/bin/sh"), true)
        .expect("failed to open /sbin/init");
    let file = match inode {
        INode::FileLike(file) => file,
        _ => panic!("/sbin/init is not a file"),
    };
    Process::new_init_process(file, console).unwrap();
}
