use crate::process::Process;
use crate::result::*;
use alloc::sync::Arc;
use x86::current::segmentation::wrfsbase;

use super::{SpinLock, UserVAddr};

const ARCH_SET_FS: i32 = 0x1002;

pub fn arch_prctl(current: &Arc<SpinLock<Process>>, code: i32, uaddr: UserVAddr) -> Result<()> {
    match code {
        // TODO: Move to arch directory.
        ARCH_SET_FS => {
            let value = uaddr.value() as u64;
            current.lock().arch.fsbase = value;
            unsafe {
                wrfsbase(value);
            }
        }
        _ => {
            return Err(Errno::EINVAL.into());
        }
    }

    Ok(())
}
