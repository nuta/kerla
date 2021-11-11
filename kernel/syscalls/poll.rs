use core::mem::size_of;

use kerla_runtime::address::UserVAddr;

use crate::{
    ctypes::{c_int, c_nfds, c_short},
    fs::{inode::PollStatus, opened_file::Fd},
    poll::POLL_WAIT_QUEUE,
    prelude::*,
    process::current_process,
    syscalls::SyscallHandler,
    timer::read_monotonic_clock,
    user_buffer::UserBuffer,
};

use crate::user_buffer::UserBufReader;

impl<'a> SyscallHandler<'a> {
    pub fn sys_poll(&mut self, fds: UserVAddr, nfds: c_nfds, timeout: c_int) -> Result<isize> {
        let started_at = read_monotonic_clock();
        POLL_WAIT_QUEUE.sleep_signalable_until(|| {
            if timeout > 0 && started_at.elapsed_msecs() >= (timeout as usize) {
                return Ok(Some(0));
            }

            // Check the statuses of all specified files one by one.
            let mut ready_fds = 0;
            let fds_len = (nfds as usize) * (size_of::<Fd>() + 2 * size_of::<c_short>());
            let mut reader = UserBufReader::from(UserBuffer::from_uaddr(fds, fds_len));
            for _ in 0..nfds {
                let fd = reader.read::<Fd>()?;
                let events = bitflags_from_user!(PollStatus, reader.read::<c_short>()?)?;

                let revents = if fd.as_int() < 0 || events.is_empty() {
                    0
                } else {
                    let status = current_process().opened_files().lock().get(fd)?.poll()?;

                    let revents = events & status;
                    if !revents.is_empty() {
                        ready_fds += 1;
                    }

                    revents.bits()
                };

                // Update revents.
                fds.add(reader.pos()).write::<c_short>(&revents)?;

                // Skip revents in the reader.
                reader.skip(size_of::<c_short>())?;
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
