use super::{printchar, serial};
use crate::boot::boot_kernel;

#[no_mangle]
pub unsafe extern "C" fn init() -> ! {
    serial::init();
    printchar('\n');
    boot_kernel();
    loop {}
}

#[no_mangle]
pub extern "C" fn mpinit() -> ! {
    loop {}
}
