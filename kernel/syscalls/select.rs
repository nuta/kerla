use kerla_runtime::address::UserVAddr;
use kerla_utils::alignment::align_up;

use crate::{
    ctypes::c_int,
    fs::{inode::PollStatus, opened_file::Fd},
    poll::POLL_WAIT_QUEUE,
    prelude::*,
    process::current_process,
    syscalls::SyscallHandler,
    timer::{read_monotonic_clock, Timeval},
};

fn check_fd_statuses<F>(max_fd: c_int, fds: UserVAddr, is_ready: F) -> Result<isize>
where
    F: Fn(PollStatus) -> bool,
{
    let num_bytes = align_up(max_fd as usize, 8) / 8;
    if num_bytes > 1024 {
        return Err(Errno::ENOMEM.into());
    }

    let mut fds_vec = vec![0; num_bytes];
    fds.read_bytes(fds_vec.as_mut_slice())?;

    let mut ready_fds = 0;
    for (byte_i, byte) in fds_vec.iter_mut().enumerate().take(num_bytes) {
        for bit_i in 0..8 {
            let fd = Fd::new((byte_i * 8 + bit_i) as c_int);
            if *byte & (1 << bit_i) != 0 && fd.as_int() <= max_fd {
                let status = current_process().opened_files().lock().get(fd)?.poll()?;

                if is_ready(status) {
                    ready_fds += 1;
                } else {
                    *byte &= !(1 << bit_i);
                }
            }
        }
    }

    if ready_fds > 0 {
        fds.write_bytes(&fds_vec)?;
    }

    Ok(ready_fds)
}

impl<'a> SyscallHandler<'a> {
    pub fn sys_select(
        &mut self,
        max_fd: c_int,
        readfds: Option<UserVAddr>,
        writefds: Option<UserVAddr>,
        _errorfds: Option<UserVAddr>,
        timeout: Option<Timeval>,
    ) -> Result<isize> {
        let started_at = read_monotonic_clock();
        let timeout_ms = timeout.map(|timeval| timeval.as_msecs());
        POLL_WAIT_QUEUE.sleep_signalable_until(|| {
            match timeout_ms {
                Some(timeout_ms) if started_at.elapsed_msecs() >= timeout_ms => {
                    return Ok(Some(0));
                }
                _ => {}
            }

            // Check the statuses of all specified files one by one.
            // TODO: Support errorfds
            let mut ready_fds = 0;
            if let Some(fds) = readfds {
                ready_fds +=
                    check_fd_statuses(max_fd, fds, |status| status.contains(PollStatus::POLLIN))?;
            }
            if let Some(fds) = writefds {
                ready_fds +=
                    check_fd_statuses(max_fd, fds, |status| status.contains(PollStatus::POLLIN))?;
            }

            if ready_fds > 0 {
                Ok(Some(ready_fds))
            } else {
                // Sleep until any changes in files or sockets occur...
                Ok(None)
            }
        })
    }
}
