mod arch_prctl;
mod process;

pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 256;
pub const USER_VALLOC_END: UserVAddr = unsafe { UserVAddr::new_unchecked(0x0000_0fff_0000_0000) };
pub const USER_VALLOC_BASE: UserVAddr = unsafe { UserVAddr::new_unchecked(0x0000_000a_0000_0000) };
pub const USER_STACK_TOP: UserVAddr = USER_VALLOC_BASE;
pub const PAGE_SIZE: usize = 4096;
pub const TICK_HZ: usize = 1000;

pub use arch_prctl::arch_prctl;
pub use process::{switch_thread, Process};

pub use kerla_runtime::arch::*;
pub use kerla_runtime::*;
