use super::*;
use crate::{
    arch::{self, disable_interrupt, enable_interrupt, is_interrupt_enabled, SpinLock, VAddr},
    elf::{Elf, ProgramHeader},
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
        page_allocator::{alloc_pages, AllocPageFlags},
        vm::{Vm, VmAreaType},
    },
    result::{Errno, Error, ErrorExt, Result},
};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::{SpinLockGuard, UserVAddr, KERNEL_STACK_SIZE, PAGE_SIZE, USER_STACK_TOP};
use arrayvec::ArrayVec;
use core::cmp::max;
use core::mem::{self, size_of, size_of_val};
use core::sync::atomic::{AtomicI32, Ordering};
use goblin::elf64::program_header::PT_LOAD;
use opened_file::OpenedFileTable;
use penguin_utils::once::Once;
use penguin_utils::{alignment::align_up, lazy::Lazy};

static NEXT_PID: AtomicI32 = AtomicI32::new(1);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PId(i32);

impl PId {
    pub const fn new(pid: i32) -> PId {
        PId(pid)
    }
}

fn alloc_pid() -> Option<PId> {
    Some(PId::new(NEXT_PID.fetch_add(1, Ordering::SeqCst)))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProcessState {
    Runnable,
    Sleeping,
}

pub struct ProcessInner {
    pub arch: arch::Thread,
    pub state: ProcessState,
}

pub struct Process {
    pub pid: PId,
    pub vm: Option<Arc<SpinLock<Vm>>>,
    pub opened_files: SpinLock<OpenedFileTable>,
    pub(super) inner: SpinLock<ProcessInner>,
}

impl Process {
    pub fn new_kthread(ip: VAddr) -> Result<Arc<Process>> {
        let stack_bottom = alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE, AllocPageFlags::KERNEL)
            .into_error_with_message(Errno::ENOMEM, "failed to allocate kernel stack")?;
        let sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);
        let process = Arc::new(Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_kthread(ip, sp),
                state: ProcessState::Runnable,
            }),
            vm: None,
            pid: alloc_pid().into_error_with_message(Errno::EAGAIN, "failed to allocate PID")?,
            opened_files: SpinLock::new(OpenedFileTable::new()),
        });

        SCHEDULER.lock().enqueue(process.clone());
        Ok(process)
    }

    pub fn new_idle_thread() -> Result<Arc<Process>> {
        Ok(Arc::new(Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_idle_thread(),
                state: ProcessState::Runnable,
            }),
            vm: None,
            pid: PId::new(0),
            opened_files: SpinLock::new(OpenedFileTable::new()),
        }))
    }

    pub fn new_init_process(
        executable: Arc<dyn FileLike>,
        root_fs: RootFs,
    ) -> Result<Arc<Process>> {
        // Read the E\LF header in the executable file.
        let file_header_len = PAGE_SIZE;
        let file_header_top = USER_STACK_TOP;
        let file_header_pages = alloc_pages(file_header_len / PAGE_SIZE, AllocPageFlags::KERNEL)
            .into_error(Errno::ENOMEM)?;
        let buf = unsafe {
            core::slice::from_raw_parts_mut(file_header_pages.as_mut_ptr(), file_header_len)
        };
        executable.read(0, buf)?;

        let elf = Elf::parse(&buf);
        let ip = elf.entry()?;
        let sp = UserVAddr::new(0xdead_0000_beef_beef)?;

        let mut end_of_image = 0;
        for phdr in elf.program_headers() {
            if phdr.p_type == PT_LOAD {
                end_of_image = max(end_of_image, (phdr.p_vaddr + phdr.p_memsz) as usize);
            }
        }

        // Set up the user stack.
        let argv = &["sh".as_bytes()];
        let envp = &[];
        let auxv = &[
            Auxv::Phdr(
                file_header_top
                    .sub(file_header_len)?
                    .add(elf.header().e_phoff as usize)?,
            ),
            Auxv::Phnum(elf.program_headers().len()),
            Auxv::Phent(size_of::<ProgramHeader>()),
            Auxv::Pagesz(PAGE_SIZE),
        ];
        const USER_STACK_LEN: usize = 128 * 1024; // TODO: Implement rlimit
        let init_stack_top = file_header_top.sub(file_header_len)?;
        let user_stack_bottom = init_stack_top.sub(USER_STACK_LEN).unwrap().value();
        let user_heap_bottom = align_up(end_of_image, PAGE_SIZE);
        let init_stack_len = align_up(estimate_user_init_stack_size(argv, envp, auxv), PAGE_SIZE);
        if user_heap_bottom >= user_stack_bottom || init_stack_len >= USER_STACK_LEN {
            return Err(Error::new(Errno::E2BIG));
        }

        let init_stack_pages = alloc_pages(init_stack_len / PAGE_SIZE, AllocPageFlags::KERNEL)
            .into_error(Errno::ENOMEM)?;
        let user_sp = init_user_stack(
            init_stack_top,
            init_stack_pages.as_vaddr().add(init_stack_len),
            init_stack_pages.as_vaddr(),
            argv,
            envp,
            auxv,
        )?;

        let mut vm = Vm::new(
            UserVAddr::new(user_stack_bottom).unwrap(),
            UserVAddr::new(user_heap_bottom).unwrap(),
        );
        for i in 0..(file_header_len / PAGE_SIZE) {
            vm.page_table_mut().map_user_page(
                file_header_top
                    .sub(((file_header_len / PAGE_SIZE) - i) * PAGE_SIZE)
                    .unwrap(),
                file_header_pages.add(i * PAGE_SIZE),
            );
        }

        for i in 0..(init_stack_len / PAGE_SIZE) {
            vm.page_table_mut().map_user_page(
                init_stack_top
                    .sub(((init_stack_len / PAGE_SIZE) - i) * PAGE_SIZE)
                    .unwrap(),
                init_stack_pages.add(i * PAGE_SIZE),
            );
        }

        // Register program headers in the virtual memory space.
        for phdr in elf.program_headers() {
            if phdr.p_type != PT_LOAD {
                continue;
            }

            let area_type = if phdr.p_filesz > 0 {
                VmAreaType::File {
                    file: executable.clone(),
                    offset: phdr.p_offset as usize,
                    file_size: phdr.p_filesz as usize,
                }
            } else {
                VmAreaType::Anonymous
            };

            vm.add_vm_area(
                UserVAddr::new(phdr.p_vaddr as usize)?,
                phdr.p_memsz as usize,
                area_type,
            );
        }

        let stack_bottom = alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE, AllocPageFlags::KERNEL)
            .into_error_with_message(Errno::ENOMEM, "failed to allocate kernel stack")?;

        let kernel_sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);

        // Open stdin.
        let mut opened_files = OpenedFileTable::new();
        let console = root_fs
            .lookup_inode(root_fs.root_dir()?, Path::new("/dev/console"), true)
            .expect("failed to open /dev/console");
        opened_files.open_with_fixed_fd(
            Fd::new(0),
            Arc::new(OpenedFile::new(console.clone(), OpenMode::O_RDONLY, 0)),
        );
        // Open stdout.
        opened_files.open_with_fixed_fd(
            Fd::new(1),
            Arc::new(OpenedFile::new(console.clone(), OpenMode::O_WRONLY, 0)),
        );
        // Open stderr.
        opened_files.open_with_fixed_fd(
            Fd::new(2),
            Arc::new(OpenedFile::new(console, OpenMode::O_WRONLY, 0)),
        );

        let process = Arc::new(Process {
            inner: SpinLock::new(ProcessInner {
                arch: arch::Thread::new_user_thread(ip, user_sp, kernel_sp),
                state: ProcessState::Runnable,
            }),
            vm: Some(Arc::new(SpinLock::new(vm))),
            pid: PId::new(1),
            opened_files: SpinLock::new(opened_files),
        });

        SCHEDULER.lock().enqueue(process.clone());
        Ok(process)
    }

    pub fn is_idle(&self) -> bool {
        self.pid == PId::new(0)
    }

    pub fn inner(&self) -> SpinLockGuard<'_, ProcessInner> {
        self.inner.lock()
    }

    pub fn vm(&self) -> SpinLockGuard<'_, Vm> {
        self.vm.as_ref().expect("not a user process").lock()
    }
}
