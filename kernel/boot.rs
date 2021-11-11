#![cfg_attr(test, allow(unreachable_code))]

use crate::{
    arch::{idle, SpinLock},
    drivers,
    fs::{devfs::SERIAL_TTY, tmpfs},
    fs::{
        devfs::{self, DEV_FS},
        initramfs::{self, INITRAM_FS},
        mount::RootFs,
        path::Path,
    },
    interrupt, logger, net, pipe, poll,
    process::{self, signal, switch, Process},
    profile::StopWatch,
    syscalls::SyscallHandler,
};
use alloc::sync::Arc;
use kerla_arch::bootinfo::BootInfo;
use kerla_utils::once::Once;
use tmpfs::TMP_FS;

#[cfg(test)]
use crate::test_runner::end_tests;

fn idle_thread() -> ! {
    loop {
        idle();
    }
}

struct Handler;

impl kerla_arch::Handler for Handler {
    fn handle_console_rx(&self, ch: u8) {
        SERIAL_TTY.input_char(ch);
    }

    fn handle_irq(&self, irq: u8) {
        crate::interrupt::handle_irq(irq);
    }

    fn handle_timer_irq(&self) {
        crate::timer::handle_timer_irq();
    }

    fn handle_page_fault(
        &self,
        unaligned_vaddr: Option<kerla_arch::UserVAddr>,
        ip: usize,
        reason: kerla_arch::PageFaultReason,
    ) {
        match unaligned_vaddr {
            Some(vaddr) => {
                crate::mm::page_fault::handle_page_fault(vaddr, ip, reason);
            }
            None => {
                // TODO: Print vaddr
                debug_warn!(
                    "user tried to access a kernel address, killing the current process...",
                );
                Process::exit_by_signal(signal::SIGSEGV);
            }
        }
    }

    fn handle_syscall(
        &self,
        a1: usize,
        a2: usize,
        a3: usize,
        a4: usize,
        a5: usize,
        a6: usize,
        n: usize,
        frame: *mut kerla_arch::SyscallFrame,
    ) -> isize {
        let mut handler = SyscallHandler::new(unsafe { &mut *frame });
        handler
            .dispatch(a1, a2, a3, a4, a5, a6, n)
            .unwrap_or_else(|err| -(err.errno() as isize))
    }

    #[cfg(debug_assertions)]
    fn usercopy_hook(&self) {
        use crate::process::current_process;

        // We should not hold the vm lock since we'll try to acquire it in the
        // page fault handler when copying caused a page fault.
        debug_assert!(!current_process().vm().as_ref().unwrap().is_locked());
    }
}

pub static INITIAL_ROOT_FS: Once<Arc<SpinLock<RootFs>>> = Once::new();

#[no_mangle]
pub fn boot_kernel(#[cfg_attr(debug_assertions, allow(unused))] bootinfo: &BootInfo) -> ! {
    logger::init();

    info!("Booting Kerla...");
    let mut profiler = StopWatch::start();

    kerla_arch::set_handler(&Handler);

    // Initialize memory allocators first.
    interrupt::init();
    profiler.lap_time("global interrupt init");

    #[cfg(test)]
    {
        crate::test_main();
        end_tests();
    }

    // Initialize kernel subsystems.
    pipe::init();
    profiler.lap_time("pipe init");
    poll::init();
    profiler.lap_time("poll init");
    devfs::init();
    profiler.lap_time("devfs init");
    tmpfs::init();
    profiler.lap_time("tmpfs init");
    initramfs::init();
    profiler.lap_time("initramfs init");
    drivers::init();
    profiler.lap_time("drivers init");

    if bootinfo.pci_enabled {
        drivers::pci::init();
        profiler.lap_time("pci init");
    }

    if !bootinfo.virtio_mmio_devices.is_empty() {
        drivers::virtio::init(&bootinfo.virtio_mmio_devices);
        profiler.lap_time("virtio init");
    }

    net::init();
    profiler.lap_time("net init");

    // Prepare the root file system.
    let mut root_fs = RootFs::new(INITRAM_FS.clone()).unwrap();
    let dev_dir = root_fs
        .lookup_dir(Path::new("/dev"))
        .expect("failed to locate /dev");
    let tmp_dir = root_fs
        .lookup_dir(Path::new("/tmp"))
        .expect("failed to locate /tmp");
    root_fs
        .mount(dev_dir, DEV_FS.clone())
        .expect("failed to mount devfs");
    root_fs
        .mount(tmp_dir, TMP_FS.clone())
        .expect("failed to mount tmpfs");

    // Open /dev/console for the init process.
    let console = root_fs
        .lookup_path(Path::new("/dev/console"), true)
        .expect("failed to open /dev/console");

    // Open the init's executable.
    let argv0 = if option_env!("INIT_SCRIPT").is_some() {
        "/bin/sh"
    } else {
        "/sbin/init"
    };
    let executable_path = root_fs
        .lookup_path(Path::new(argv0), true)
        .expect("failed to open the init executable");

    // We cannot initialize the process subsystem until INITIAL_ROOT_FS is initialized.
    INITIAL_ROOT_FS.init(|| Arc::new(SpinLock::new(root_fs)));

    profiler.lap_time("root fs init");

    process::init();
    profiler.lap_time("process init");

    // Create the init process.
    if let Some(script) = option_env!("INIT_SCRIPT") {
        let argv = &[b"sh", b"-c", script.as_bytes()];
        info!("running init script: {:?}", script);
        Process::new_init_process(INITIAL_ROOT_FS.clone(), executable_path, console, argv)
            .expect("failed to execute the init script: ");
    } else {
        info!("running /sbin/init");
        Process::new_init_process(
            INITIAL_ROOT_FS.clone(),
            executable_path,
            console,
            &[b"/sbin/init"],
        )
        .expect("failed to execute /sbin/init");
    }

    profiler.lap_time("first process init");

    // We've done the kernel initialization. Switch into the init...
    switch();

    // We're now in the idle thread context.
    idle_thread();
}
