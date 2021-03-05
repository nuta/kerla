macro_rules! __cpu_local_impl {
    ($V:vis, $N:ident, $T:ty, $E:expr) => {
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
            $V fn vaddr(&self) -> $crate::arch::x64::VAddr {
                extern "C" {
                    static __cpu_local: u8;
                }

                unsafe {
                    let cpu_local_base = &__cpu_local as *const _ as usize;
                    let offset = (self as *const _ as usize) - cpu_local_base;
                    let gsbase = x86::bits64::segmentation::rdgsbase() as usize;
                    $crate::arch::x64::VAddr::new((gsbase + offset) as u64)
                }
            }
        }

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
macro_rules! cpu_local {
    (static ref $N:ident : $T:ty = $E:expr ;) => {
        __cpu_local_impl!(, $N, $T, $E);
    };
    (pub static ref $N:ident : $T:ty = $E:expr ;) => {
        __cpu_local_impl!(pub, $N, $T, $E);
    };
}
