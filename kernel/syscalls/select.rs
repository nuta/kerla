use penguin_utils::alignment::align_up;

use crate::{
    arch::UserVAddr,
    ctypes::c_int,
    fs::{inode::PollStatus, opened_file::Fd},
    poll::POLL_WAIT_QUEUE,
    result::Result,
    timer::read_monotonic_clock,
};
use crate::{process::current_process, syscalls::SyscallDispatcher};

use super::{Timeval, UserBufReader};

fn check_fd_statuses<F>(num_fds: c_int, fds: UserVAddr, is_ready: F) -> Result<isize>
where
    F: Fn(PollStatus) -> bool,
{
    if fds.is_null() {
        return Ok(0);
    }

    let mut ready_fds = 0;
    let mut reader = UserBufReader::new(fds);
    for byte_i in 0..(align_up(num_fds as usize, 8) / 8) {
        let mut bitmap: u8 = reader.read()?;
        for bit_i in 0..8 {
            let fd = Fd::new((byte_i * 8 + bit_i) as c_int);
            if bitmap & (1 << bit_i) != 0 && fd.as_int() < num_fds {
                let status = current_process()
                    .opened_files
                    .lock()
                    .get(fd)?
                    .lock()
                    .poll()?;

                if is_ready(status) {
                    ready_fds += 1;
                } else {
                    bitmap &= !(1 << bit_i);
                }
            }
        }

        fds.add(reader.pos() - 1)?.write::<u8>(&bitmap)?;
    }

    Ok(ready_fds)
}

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_select(
        &mut self,
        nfds: c_int,
        readfds: UserVAddr,
        writefds: UserVAddr,
        _errorfds: UserVAddr,
        timeout: Option<Timeval>,
    ) -> Result<isize> {
        let started_at = read_monotonic_clock();
        let timeout_ms = timeout.map(|timeval| timeval.as_msecs());
        POLL_WAIT_QUEUE.sleep_until(|| {
            match timeout_ms {
                Some(timeout_ms) if started_at.elapsed_msecs() >= timeout_ms => {
                    return Ok(Some(0));
                }
                _ => {}
            }

            // Check the statuses of all specified files one by one.
            // TODO: Support errorfds
            let ready_fds =
                check_fd_statuses(nfds, readfds, |status| status.contains(PollStatus::POLLIN))?
                    + check_fd_statuses(nfds, writefds, |status| {
                        status.contains(PollStatus::POLLOUT)
                    })?;

            if ready_fds > 0 {
                Ok(Some(ready_fds))
            } else {
                // Sleep until any changes in files or sockets occur...
                Ok(None)
            }
        })
    }
}
