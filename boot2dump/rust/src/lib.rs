//! A library for OS developers to save a kernel crash log happened in your
//! own kernel into a file on disk reboot the computer.
//!
//! # Prerequisites
//!
//! You kernel needs to satisfy the following prerequisites to use this crate:
//!
//! - The CPU is x86_64 and is in the 64-bit mode.
//! - The file system is ext4 and its on a virtio-blk device.
//! - A **sufficiently large** file for the crash log (e.g. in the following example, `kerla.dump`)
//!   already exists in the **root directory**.
//! - Virtual addresses starting `0xffff_8000_0000_0000` are straight mapped into
//!   from the physical addresses 0 (i.e. `0xffff_8000_0001_0000` points to `0x1_0000`).
//!   It should cover the memory pages where boot2dump image exist.
//!
//! # How to Use
//!
//! The usage is pretty simple: at the end of your panic handler, call
//! [`save_to_file_and_reboot`]. It will save the given buffer into a file
//! and then reboot the computer.
//!
//! # Example
//!
//! ```ignore
//! use boot2dump::save_to_file_and_reboot;
//!
//! #[panic_handler]
//! fn panic(info: &core::panic::PanicInfo) -> ! {
//!     // Save the panic message into a file. Let's hope `format!` won't panic...
//!     let message = format!("{}", info).as_bytes();
//!     save_to_file_and_reboot("kerla.dump",  message.as_bytes());
//! }
//! ```
#![no_std]

#[repr(align(4096))]
struct PageAligned;

// A quick workaround for aligning the image location to a page boundary.
// https://users.rust-lang.org/t/can-i-conveniently-compile-bytes-into-a-rust-program-with-a-specific-alignment/24049/2
#[repr(C)]
struct Image<T: ?Sized> {
    _align: [PageAligned; 0],
    data: T,
}

static BOOT2DUMP: &'static Image<[u8]> = &Image {
    _align: [],
    data: *include_bytes!("../boot2dump.bin"),
};

/// Saves `data` into `filename` on the disk and then reboots the computer.
///
/// Currently, it only supports saving to a file in the root directory. Thus,
/// `filename` should be a filename without slashes (`/`), for example,
/// `kerla.dump`.
///
/// # Safety
///
/// This function will boot another operating system (boot2dump) and it may cause
/// a problem.
pub unsafe fn save_to_file_and_reboot(filename: &str, data: &[u8]) -> ! {
    type EntryPoint = extern "C" fn(*const u8, u64, *const u8, u64);
    let entrypoint = core::mem::transmute::<_, EntryPoint>(BOOT2DUMP.data.as_ptr());
    entrypoint(
        filename.as_ptr(),
        filename.len() as u64,
        data.as_ptr(),
        data.len() as u64,
    );
    core::hint::unreachable_unchecked();
}
