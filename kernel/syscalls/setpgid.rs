use crate::{
    prelude::*,
    process::process_group::PgId,
    process::{current_process, process_group::ProcessGroup, PId, Process},
    result::Result,
    syscalls::SyscallHandler,
};

impl<'a> SyscallHandler<'a> {
    pub fn sys_setpgid(&mut self, pid: PId, pgid: PgId) -> Result<isize> {
        let current = if pid.as_i32() == 0 {
            current_process().clone()
        } else {
            Process::find_by_pid(pid).ok_or_else(|| Error::new(Errno::ESRCH))?
        };

        let new_pg = ProcessGroup::find_or_create_by_pgid(pgid);
        let proc_weak = Arc::downgrade(&current);
        let old_pg = current.process_group();

        if !Arc::ptr_eq(&old_pg, &new_pg) {
            old_pg.lock().remove(&proc_weak);
            new_pg.lock().add(proc_weak);
            current.set_process_group(Arc::downgrade(&new_pg));
        }

        Ok(pgid.as_i32() as isize)
    }
}
