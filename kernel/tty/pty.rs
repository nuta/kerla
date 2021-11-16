//! Pseudo-terminal (PTY).

use core::{cmp::min, fmt};

use alloc::sync::Arc;
use alloc::vec::Vec;
use kerla_runtime::spinlock::SpinLock;
use kerla_utils::id_table::IdTable;

use crate::{
    fs::{
        inode::{FileLike, INodeNo, PollStatus},
        opened_file::OpenOptions,
        stat::{FileMode, Stat, S_IFCHR},
        tmpfs,
    },
    poll::POLL_WAIT_QUEUE,
    process::WaitQueue,
    result::{Errno, Error, Result},
    user_buffer::{UserBufReader, UserBufWriter, UserBuffer, UserBufferMut},
};

use super::line_discipline::{LineControl, LineDiscipline};

static PTY_INDEX_TABLE: SpinLock<IdTable<16>> = SpinLock::new(IdTable::new());

pub struct PtyMaster {
    index: usize,
    wait_queue: WaitQueue,
    buf: SpinLock<Vec<u8>>,
    discipline: LineDiscipline,
}

impl PtyMaster {
    pub fn new() -> Result<(Arc<PtyMaster>, Arc<PtySlave>)> {
        let master = Arc::new(PtyMaster {
            index: PTY_INDEX_TABLE
                .lock()
                .alloc()
                .ok_or_else(|| Error::new(Errno::ENOMEM))?,
            wait_queue: WaitQueue::new(),
            buf: SpinLock::new(Vec::new()),
            discipline: LineDiscipline::new(),
        });

        let slave = Arc::new(PtySlave::new(master.clone()));
        Ok((master, slave))
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

impl Drop for PtyMaster {
    fn drop(&mut self) {
        PTY_INDEX_TABLE.lock().free(self.index);
    }
}

impl FileLike for PtyMaster {
    fn read(
        &self,
        _offset: usize,
        buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        let mut writer = UserBufWriter::from(buf);
        let read_len = self.wait_queue.sleep_signalable_until(|| {
            let mut buf_lock = self.buf.lock();
            if buf_lock.is_empty() {
                // TODO: NOBLOCK
                return Ok(None);
            }

            let copy_len = min(buf_lock.len(), writer.remaining_len());
            writer.write_bytes(&buf_lock[..copy_len])?;
            buf_lock.drain(..copy_len);
            Ok(Some(copy_len))
        })?;

        if read_len > 0 {
            POLL_WAIT_QUEUE.wake_all();
        }

        Ok(read_len)
    }

    fn write(&self, _offset: usize, buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        let written_len = self.discipline.write(buf, |ctrl| {
            let mut master_buf = self.buf.lock();
            match ctrl {
                LineControl::Backspace => {
                    // Remove the previous character by overwriting with a whitespace.
                    master_buf.extend_from_slice(b"\x08 \x08");
                }
                LineControl::Echo(ch) => {
                    master_buf.push(ch);
                }
            }
        })?;

        if written_len > 0 {
            POLL_WAIT_QUEUE.wake_all();
        }

        Ok(written_len)
    }

    fn ioctl(&self, cmd: usize, _arg: usize) -> Result<isize> {
        debug_warn!("pty_master: unknown cmd={:x}", cmd);
        Ok(0)
    }

    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(5), // FIXME:
            mode: FileMode::new(S_IFCHR | 0o666),
            ..Stat::zeroed()
        })
    }

    fn poll(&self) -> Result<PollStatus> {
        let mut status = PollStatus::empty();

        if !self.buf.lock().is_empty() {
            status |= PollStatus::POLLIN;
        }

        if self.discipline.is_writable() {
            status |= PollStatus::POLLOUT;
        }

        Ok(status)
    }
}

impl fmt::Debug for PtyMaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PtyMaster")
            .field("index", &self.index)
            .finish()
    }
}

pub struct PtySlave {
    master: Arc<PtyMaster>,
}

impl PtySlave {
    pub fn new(master: Arc<PtyMaster>) -> PtySlave {
        PtySlave { master }
    }
}

impl FileLike for PtySlave {
    fn read(
        &self,
        _offset: usize,
        buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        let read_len = self.master.discipline.read(buf)?;
        if read_len > 0 {
            POLL_WAIT_QUEUE.wake_all();
        }
        Ok(read_len)
    }

    fn write(&self, _offset: usize, buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        let mut written_len = 0;
        let mut master_buf = self.master.buf.lock();
        let mut reader = UserBufReader::from(buf);
        while reader.remaining_len() > 0 {
            let mut tmp = [0; 128];
            let copied_len = reader.read_bytes(&mut tmp)?;
            for ch in &tmp[..copied_len] {
                // FIXME: Block if the buffer become too large.
                // TODO: check termios
                match *ch {
                    b'\n' => {
                        // ONLCR: Convert NL to CR + NL
                        master_buf.push(b'\r');
                        master_buf.push(b'\n');
                    }
                    _ => {
                        master_buf.push(*ch);
                    }
                }
            }

            written_len += copied_len;
        }

        if written_len > 0 {
            POLL_WAIT_QUEUE.wake_all();
        }
        Ok(written_len)
    }

    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(6), // FIXME:
            mode: FileMode::new(S_IFCHR | 0o666),
            ..Stat::zeroed()
        })
    }

    fn ioctl(&self, cmd: usize, _arg: usize) -> Result<isize> {
        const TIOCSPTLCK: usize = 0x40045431;
        match cmd {
            TIOCSPTLCK => Ok(0),
            _ => {
                debug_warn!("pty_slave: unknown cmd={:x}", cmd);
                Ok(0)
            }
        }
    }

    fn poll(&self) -> Result<PollStatus> {
        let mut status = PollStatus::empty();

        if self.master.discipline.is_readable() {
            status |= PollStatus::POLLIN;
        }

        // TODO: if self.master.discipline.lock().len() > FULL {
        status |= PollStatus::POLLOUT;

        Ok(status)
    }
}

impl fmt::Debug for PtySlave {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PtySlave")
            .field("master", &self.master.index)
            .finish()
    }
}

pub struct Ptmx {
    pts_dir: Arc<tmpfs::Dir>,
}

impl Ptmx {
    pub fn new(pts_dir: Arc<tmpfs::Dir>) -> Ptmx {
        Ptmx { pts_dir }
    }
}

impl FileLike for Ptmx {
    fn open(&self, _options: &OpenOptions) -> Result<Option<Arc<dyn FileLike>>> {
        let (master, slave) = PtyMaster::new()?;
        self.pts_dir.add_file(&format!("{}", master.index()), slave);
        Ok(Some(master as Arc<dyn FileLike>))
    }

    fn stat(&self) -> Result<Stat> {
        Ok(Stat {
            inode_no: INodeNo::new(4),
            mode: FileMode::new(S_IFCHR | 0o666),
            ..Stat::zeroed()
        })
    }

    fn read(
        &self,
        _offset: usize,
        _buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        unreachable!();
    }

    fn write(&self, _offset: usize, _buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        unreachable!();
    }

    fn poll(&self) -> Result<PollStatus> {
        let status = PollStatus::empty();
        // TODO: What should we return?
        Ok(status)
    }
}

impl fmt::Debug for Ptmx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ptmx").finish()
    }
}
