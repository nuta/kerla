#![cfg_attr(test, allow(unreachable_code))]

use crate::{
    arch::{self, idle, PAddr, SpinLock},
    drivers,
    fs::tmpfs,
    fs::{
        devfs::{self, DEV_FS},
        initramfs::{self, INITRAM_FS},
        mount::RootFs,
        path::Path,
    },
    mm::{global_allocator, page_allocator},
    net,
    printk::PrintkLogger,
    process::{self, switch, Process, ProcessState},
};
use alloc::sync::Arc;
use penguin_utils::once::Once;
use tmpfs::TMP_FS;

#[cfg(test)]
use crate::test_runner::end_tests;
use arrayvec::ArrayVec;

pub struct RamArea {
    pub base: PAddr,
    pub len: usize,
}

pub struct BootInfo {
    pub ram_areas: ArrayVec<[RamArea; 8]>,
}

static LOGGER: PrintkLogger = PrintkLogger;

pub fn init_logger() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(if cfg!(debug_assertions) {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    });
}

fn idle_thread() -> ! {
    loop {
        idle();
    }
}

pub static INITIAL_ROOT_FS: Once<Arc<SpinLock<RootFs>>> = Once::new();

pub fn boot_kernel(bootinfo: &BootInfo) -> ! {
    // Initialize memory allocators first.
    page_allocator::init(&bootinfo.ram_areas);
    global_allocator::init();

    #[cfg(test)]
    {
        crate::test_main();
        end_tests();
    }

    // Initialize kernel subsystems.
    arch::init();
    devfs::init();
    tmpfs::init();
    initramfs::init();
    drivers::init();
    net::init();

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
        .lookup_file(Path::new("/dev/console"))
        .expect("failed to open /dev/console")
        .into();

    // Open the init's executable.
    // FIXME: We use /bin/sh for now.
    let executable = root_fs
        .lookup_file(Path::new("/bin/sh"))
        .expect("failed to open /sbin/init");

    // We cannot initialize the process subsystem until INITIAL_ROOT_FS is initialized.
    INITIAL_ROOT_FS.init(|| Arc::new(SpinLock::new(root_fs)));
    process::init();

    // Create the init process.
    Process::new_init_process(
        INITIAL_ROOT_FS.clone(),
        executable,
        console,
        &["/bin/sh".as_bytes()],
    )
    .expect("failed to execute /sbin/init");

    // We've done the kernel initialization. Switch into the init...
    switch(ProcessState::Runnable);

    // We're now in the idle thread context.
    idle_thread();
}
