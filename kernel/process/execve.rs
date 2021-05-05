use super::elf::{Elf, ProgramHeader};
use crate::mm::page_allocator::{alloc_pages, AllocPageFlags};
use crate::process::{signal::SignalDelivery, *};
use crate::result::{Errno, Error, Result};
use crate::{
    fs::{
        mount::RootFs,
        opened_file::{OpenOptions, OpenedFileTable},
        path::Path,
    },
    random::read_secure_random,
};
use alloc::sync::Weak;
use alloc::vec::Vec;
use goblin::elf64::program_header::PT_LOAD;

pub fn execve(
    parent: Option<Weak<SpinLock<Process>>>,
    pid: PId,
    executable_path: Arc<PathComponent>,
    argv: &[&[u8]],
    envp: &[&[u8]],
    root_fs: Arc<SpinLock<RootFs>>,
    opened_files: Arc<SpinLock<OpenedFileTable>>,
) -> Result<Arc<SpinLock<Process>>> {
    do_execve(
        parent,
        pid,
        executable_path,
        argv,
        envp,
        root_fs,
        opened_files,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn do_execve(
    parent: Option<Weak<SpinLock<Process>>>,
    pid: PId,
    executable_path: Arc<PathComponent>,
    argv: &[&[u8]],
    envp: &[&[u8]],
    root_fs: Arc<SpinLock<RootFs>>,
    opened_files: Arc<SpinLock<OpenedFileTable>>,
    support_shebang: bool,
) -> Result<Arc<SpinLock<Process>>> {
    // Read the ELF header in the executable file.
    let file_header_len = PAGE_SIZE;
    let file_header_top = USER_STACK_TOP;
    let file_header_pages = alloc_pages(file_header_len / PAGE_SIZE, AllocPageFlags::KERNEL)?;
    let buf =
        unsafe { core::slice::from_raw_parts_mut(file_header_pages.as_mut_ptr(), file_header_len) };

    let executable = executable_path.inode.as_file()?;
    executable.read(0, buf.into(), &OpenOptions::readwrite())?;

    if support_shebang && buf.starts_with(b"#!") && buf.contains(&b'\n') {
        // Parse the shebang and load and overwrite argv and executable.
        let mut argv: Vec<&[u8]> = buf[2..buf.iter().position(|&ch| ch == b'\n').unwrap()]
            .split(|&ch| ch == b' ')
            .collect();
        if argv.is_empty() {
            return Err(Errno::EINVAL.into());
        }

        let executable_pathbuf = executable_path.resolve_absolute_path();
        argv.push(executable_pathbuf.as_str().as_bytes());

        // FIXME: We should use &[u8] in Path.
        let shebang_path = root_fs.lock().lookup_path(
            Path::new(core::str::from_utf8(argv[0]).map_err(|_| Error::new(Errno::EINVAL))?),
            true,
        )?;

        return do_execve(
            parent,
            pid,
            shebang_path,
            &argv,
            envp,
            root_fs,
            opened_files,
            false,
        );
    }

    let elf = Elf::parse(&buf)?;
    let ip = elf.entry()?;
    let _sp = UserVAddr::new(0xdead_0000_beef_beef)?;

    let mut end_of_image = 0;
    for phdr in elf.program_headers() {
        if phdr.p_type == PT_LOAD {
            end_of_image = max(end_of_image, (phdr.p_vaddr + phdr.p_memsz) as usize);
        }
    }

    // use core::slice::SlicePattern;
    let mut random_bytes = [0u8; 16];
    read_secure_random(((&mut random_bytes) as &mut [u8]).into())?;

    // Set up the user stack.
    let auxv = &[
        Auxv::Phdr(
            file_header_top
                .sub(file_header_len)?
                .add(elf.header().e_phoff as usize)?,
        ),
        Auxv::Phnum(elf.program_headers().len()),
        Auxv::Phent(size_of::<ProgramHeader>()),
        Auxv::Pagesz(PAGE_SIZE),
        Auxv::Random(random_bytes),
    ];
    const USER_STACK_LEN: usize = 128 * 1024; // TODO: Implement rlimit
    let init_stack_top = file_header_top.sub(file_header_len)?;
    let user_stack_bottom = init_stack_top.sub(USER_STACK_LEN).unwrap().value();
    let user_heap_bottom = align_up(end_of_image, PAGE_SIZE);
    let init_stack_len = align_up(estimate_user_init_stack_size(argv, envp, auxv), PAGE_SIZE);
    if user_heap_bottom >= user_stack_bottom || init_stack_len >= USER_STACK_LEN {
        return Err(Errno::E2BIG.into());
    }

    let init_stack_pages = alloc_pages(init_stack_len / PAGE_SIZE, AllocPageFlags::KERNEL)?;
    let user_sp = init_user_stack(
        init_stack_top,
        init_stack_pages.as_vaddr().add(init_stack_len),
        init_stack_pages.as_vaddr(),
        argv,
        envp,
        auxv,
    )?;

    let mut vm = Vm::new(
        UserVAddr::new_nonnull(user_stack_bottom).unwrap(),
        UserVAddr::new_nonnull(user_heap_bottom).unwrap(),
    )?;
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
            UserVAddr::new_nonnull(phdr.p_vaddr as usize)?,
            phdr.p_memsz as usize,
            area_type,
        )?;
    }

    let stack_bottom = alloc_pages(KERNEL_STACK_SIZE / PAGE_SIZE, AllocPageFlags::KERNEL)?;
    let kernel_sp = stack_bottom.as_vaddr().add(KERNEL_STACK_SIZE);

    opened_files.lock().close_cloexec_files();

    let process = Arc::new(SpinLock::new(Process {
        parent: parent.clone(),
        children: Vec::new(),
        state: ProcessState::Runnable,
        arch: arch::Thread::new_user_thread(ip, user_sp, kernel_sp),
        root_fs,
        vm: Some(Arc::new(SpinLock::new(vm))),
        pid,
        opened_files,
        signals: SignalDelivery::new(),
        signaled_frame: None,
    }));

    if let Some(parent) = parent.and_then(|parent| parent.upgrade()) {
        parent.lock().children.push(process.clone());
    }

    PROCESSES.lock().insert(pid, process.clone());
    SCHEDULER.lock().enqueue(pid);
    Ok(process)
}
