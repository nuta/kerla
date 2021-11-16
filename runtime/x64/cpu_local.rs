use crate::address::VAddr;
use core::mem::MaybeUninit;
use core::ptr;
use x86::bits64::segmentation::{rdgsbase, wrgsbase};

#[macro_export]
macro_rules! __cpu_local_impl {
    ($V:vis, $N:ident, $T:ty, $E:expr) => {
        #[allow(non_camel_case_types)]
        #[allow(clippy::upper_case_acronyms)]
        pub struct $N {
            #[allow(unused)]
            initial_value: $T,
        }

        impl $N {
            #[allow(unused)]
            $V fn get(&self) -> &$T {
                self.as_mut()
            }

            #[allow(unused)]
            $V fn set(&self, value: $T) {
                *self.as_mut() = value;
            }

            #[allow(unused)]
            #[allow(clippy::mut_from_ref)]
            $V fn as_mut(&self) -> &mut $T {
                unsafe { &mut *self.vaddr().as_mut_ptr() }
            }

            #[allow(unused)]
            $V fn vaddr(&self) -> $crate::address::VAddr {
                extern "C" {
                    static __cpu_local: u8;
                }

                unsafe {
                    let cpu_local_base = &__cpu_local as *const _ as usize;
                    let offset = (self as *const _ as usize) - cpu_local_base;
                    let gsbase = x86::bits64::segmentation::rdgsbase() as usize;
                    $crate::address::VAddr::new((gsbase + offset) as usize)
                }
            }
        }

        #[used]
        #[link_section = ".cpu_local"]
        $V static $N: $N = $N { initial_value: $E };
        unsafe impl Sync for $N {}
    };
}

/// Defines a CPU-local variable.
///
/// ```
/// cpu_local! {
///     pub static ref A: usize = 123;
/// }
///
/// fn init() {
///     A.set(456);
///     println!("A = {}", A.get()); // 456
/// }
/// ```
///
/// Since CPU-local variable will never be accessed from multiple CPUs at the same
/// time, it is always mutable through `.set(value)` or `.as_mut()`.
///
/// To get the memory address, use `.vaddr()`. **DO NOT USE `&` operator**  --
/// it points to the initial value area instead!
#[macro_export]
macro_rules! cpu_local {
    (static ref $N:ident : $T:ty = $E:expr ;) => {
        __cpu_local_impl!(, $N, $T, $E);
    };
    (pub static ref $N:ident : $T:ty = $E:expr ;) => {
        __cpu_local_impl!(pub, $N, $T, $E);
    };
}

/// The cpu-local structure at the beginning of the GSBASE.
#[repr(C, packed)]
pub struct CpuLocalHead {
    /// The kernel stack in the syscall context.
    pub rsp0: u64,
    /// The temporary save space for the user stack in the syscall context.
    pub rsp3: u64,
}

#[used]
#[link_section = ".cpu_local_head"]
static CPU_LOCAL_HEAD_SPACE: MaybeUninit<CpuLocalHead> = MaybeUninit::uninit();

pub fn cpu_local_head() -> &'static mut CpuLocalHead {
    unsafe { &mut *(rdgsbase() as *mut CpuLocalHead) }
}

pub unsafe fn init(cpu_local_area: VAddr) {
    extern "C" {
        static __cpu_local: u8;
        static __cpu_local_size: u8;
    }

    let template = VAddr::new(&__cpu_local as *const _ as usize);
    let len = &__cpu_local_size as *const _ as usize;
    ptr::copy_nonoverlapping::<u8>(template.as_ptr(), cpu_local_area.as_mut_ptr(), len);

    wrgsbase(cpu_local_area.value() as u64);
}
