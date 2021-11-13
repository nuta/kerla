use arrayvec::ArrayString;
use core::fmt;

use crate::{
    ctypes::*,
    fs::{
        inode::{FileLike, INodeNo},
        opened_file::OpenOptions,
        stat::{FileMode, Stat, S_IFCHR},
    },
    prelude::*,
    process::process_group::{PgId, ProcessGroup},
    result::Result,
    tty::line_discipline::*,
    user_buffer::UserBuffer,
    user_buffer::{UserBufReader, UserBufferMut},
};
use kerla_runtime::{address::UserVAddr, print::get_printer, spinlock::SpinLock};

pub struct Tty {
    name: ArrayString<8>,
    discipline: LineDiscipline,
}

impl Tty {
    pub fn new(name: &str) -> Tty {
        let mut name_buf = ArrayString::new();
        let _ = name_buf.try_push_str(name);
        Tty {
            name: name_buf,
            discipline: LineDiscipline::new(),
        }
    }

    pub fn input_char(&self, ch: u8) {
        self.discipline
            .write(([ch].as_slice()).into(), |ctrl| {
                match ctrl {
                    LineControl::Backspace => {
                        // Remove the previous character by overwriting with a whitespace.
                        get_printer().print_bytes(b"\x08 \x08");
                    }
                    LineControl::Echo(ch) => {
                        self.write(0, [ch].as_slice().into(), &OpenOptions::readwrite())
                            .ok();
                    }
                }
            })
            .ok();
    }

    pub fn set_foreground_process_group(&self, pg: Weak<SpinLock<ProcessGroup>>) {
        self.discipline.set_foreground_process_group(pg);
    }
}

const TIOCGPGRP: usize = 0x540f;
const TIOCSPGRP: usize = 0x5410;
const TIOCGWINSZ: usize = 0x5413;

impl fmt::Debug for Tty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tty").field("name", &self.name).finish()
    }
}

impl FileLike for Tty {
    fn ioctl(&self, cmd: usize, arg: usize) -> Result<isize> {
        match cmd {
            TIOCGPGRP => {
                let process_group = self
                    .discipline
                    .foreground_process_group()
                    .ok_or_else(|| Error::new(Errno::ENOENT))?;

                let pgid = process_group.lock().pgid().as_i32();
                let arg = UserVAddr::new_nonnull(arg)?;
                arg.write::<c_int>(&pgid)?;
            }
            TIOCSPGRP => {
                let arg = UserVAddr::new_nonnull(arg)?;
                let pgid = arg.read::<c_int>()?;
                let pg = ProcessGroup::find_by_pgid(PgId::new(pgid))
                    .ok_or_else(|| Error::new(Errno::ESRCH))?;
                self.discipline
                    .set_foreground_process_group(Arc::downgrade(&pg));
            }
            TIOCGWINSZ => {
                // TODO: It's not yet implemented but should return a successful
                //       value since it is used in musl's isatty(3).
            }
            _ => return Err(Errno::ENOSYS.into()),
        }

        Ok(0)
    }

    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(3),
            mode: FileMode::new(S_IFCHR | 0o666),
            ..Stat::zeroed()
        })
    }

    fn read(
        &self,
        _offset: usize,
        dst: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        self.discipline.read(dst)
    }

    fn write(&self, _offset: usize, buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        let mut tmp = [0; 32];
        let mut total_len = 0;
        let mut reader = UserBufReader::from(buf);
        while reader.remaining_len() > 0 {
            let copied_len = reader.read_bytes(&mut tmp)?;
            get_printer().print_bytes(&tmp.as_slice()[..copied_len]);
            total_len += copied_len;
        }
        Ok(total_len)
    }
}
