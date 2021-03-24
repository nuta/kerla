use crate::elf::{Elf, ProgramHeader};
use crate::fs::{mount::RootFs, opened_file::OpenedFileTable};
use crate::mm::page_allocator::{alloc_pages, AllocPageFlags};
use crate::process::*;
use crate::result::{Errno, Error, ErrorExt, Result};
use goblin::elf64::program_header::PT_LOAD;

pub fn execve(
    pid: PId,
    executable: Arc<dyn FileLike>,
    argv: &[&[u8]],
    envp: &[&[u8]],
    root_fs: Arc<SpinLock<RootFs>>,
    opened_files: Arc<SpinLock<OpenedFileTable>>,
) -> Result<Arc<Process>> {
    // Read the E\LF header in the executable file.
    let file_header_len = PAGE_SIZE;
    let file_header_top = USER_STACK_TOP;
    let file_header_pages = alloc_pages(file_header_len / PAGE_SIZE, AllocPageFlags::KERNEL)
        .into_error(Errno::ENOMEM)?;
    let buf =
        unsafe { core::slice::from_raw_parts_mut(file_header_pages.as_mut_ptr(), file_header_len) };
    executable.read(0, buf)?;

    let elf = Elf::parse(&buf);
    let ip = elf.entry()?;
    let _sp = UserVAddr::new(0xdead_0000_beef_beef)?;

    let mut end_of_image = 0;
    for phdr in elf.program_headers() {
        if phdr.p_type == PT_LOAD {
            end_of_image = max(end_of_image, (phdr.p_vaddr + phdr.p_memsz) as usize);
        }
    }

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

    let process = Arc::new(Process {
        inner: SpinLock::new(MutableFields {
            arch: arch::Thread::new_user_thread(ip, user_sp, kernel_sp),
            state: ProcessState::Runnable,
        }),
        root_fs,
        vm: Some(Arc::new(SpinLock::new(vm))),
        pid,
        opened_files,
    });

    SCHEDULER.lock().enqueue(process.clone());
    Ok(process)
}
