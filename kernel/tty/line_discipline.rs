//! Line discipline.

use crate::{
    prelude::*,
    process::{current_process, process_group::ProcessGroup, signal::SIGINT, WaitQueue},
    user_buffer::{UserBufReader, UserBufWriter, UserBuffer, UserBufferMut},
};
use bitflags::bitflags;
use kerla_runtime::spinlock::SpinLock;
use kerla_utils::ring_buffer::RingBuffer;

bitflags! {
    pub struct LFlag: u32 {
        const ICANON = 0o0000002;
        const ECHO   = 0o0000010;
    }
}

bitflags! {
    pub struct IFlag: u32 {
        const ICRNL  = 0o0000400;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Termios {
    pub lflag: LFlag,
    pub iflag: IFlag,
}

impl Termios {
    pub fn is_cooked_mode(&self) -> bool {
        self.lflag.contains(LFlag::ICANON)
    }
}

impl Default for Termios {
    fn default() -> Termios {
        Termios {
            lflag: LFlag::ICANON | LFlag::ECHO,
            iflag: IFlag::ICRNL,
        }
    }
}

// TODO: cursor
pub struct LineEdit {
    buf: Vec<u8>,
}

impl LineEdit {
    pub fn new() -> LineEdit {
        LineEdit { buf: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn insert(&mut self, ch: u8) {
        self.buf.push(ch);
    }

    pub fn backspace(&mut self) {
        self.buf.pop();
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LineControl {
    Backspace,
    Echo(u8),
}

pub struct LineDiscipline {
    wait_queue: WaitQueue,
    current_line: SpinLock<LineEdit>,
    buf: SpinLock<RingBuffer<u8, 4096>>,
    termios: SpinLock<Termios>,
    foreground_process_group: SpinLock<Weak<SpinLock<ProcessGroup>>>,
}

impl LineDiscipline {
    pub fn new() -> LineDiscipline {
        LineDiscipline {
            wait_queue: WaitQueue::new(),
            current_line: SpinLock::new(LineEdit::new()),
            buf: SpinLock::new(RingBuffer::new()),
            termios: SpinLock::new(Default::default()),
            foreground_process_group: SpinLock::new(Weak::new()),
        }
    }

    pub fn is_readable(&self) -> bool {
        self.buf.lock().is_readable()
    }

    pub fn is_writable(&self) -> bool {
        self.buf.lock().is_writable()
    }

    pub fn foreground_process_group(&self) -> Option<Arc<SpinLock<ProcessGroup>>> {
        self.foreground_process_group.lock().upgrade()
    }

    pub fn set_foreground_process_group(&self, pg: Weak<SpinLock<ProcessGroup>>) {
        *self.foreground_process_group.lock() = pg;
    }

    fn is_current_foreground(&self) -> bool {
        let foreground_pg = &*self.foreground_process_group.lock();
        current_process().belongs_to_process_group(foreground_pg)
        // If the foreground process is not yet set, allow any processes to read
        // from the tty. I'm not sure whether it is a correct behaviour.
        || foreground_pg.upgrade().is_none()
    }

    pub fn write<F>(&self, buf: UserBuffer<'_>, callback: F) -> Result<usize>
    where
        F: Fn(LineControl),
    {
        let termios = self.termios.lock();
        let mut current_line = self.current_line.lock();
        let mut ringbuf = self.buf.lock();
        let mut written_len = 0;
        let mut reader = UserBufReader::from(buf);
        while reader.remaining_len() > 0 {
            let mut tmp = [0; 128];
            let copied_len = reader.read_bytes(&mut tmp)?;
            for ch in &tmp.as_slice()[..copied_len] {
                match ch {
                    0x03 /* ETX: End of Text (^C) */  if termios.is_cooked_mode() => {
                        if let Some(pg) = self.foreground_process_group() {
                            pg.lock().signal(SIGINT);
                        }
                    }
                    0x7f /* backspace */ if termios.is_cooked_mode() => {
                        if !current_line.is_empty() {
                            current_line.backspace();
                            callback(LineControl::Backspace);
                        }
                    }
                    b'\r' if termios.iflag.contains(IFlag::ICRNL) => {
                        current_line.insert(b'\n');
                        ringbuf.push_slice(current_line.as_bytes());
                        current_line.clear();
                        if termios.lflag.contains(LFlag::ECHO) {
                            callback(LineControl::Echo(b'\r')); // FIXME: Should we echo \r?
                            callback(LineControl::Echo(b'\n'));
                        }
                    }
                    b'\n' => {
                        current_line.insert(b'\n');
                        ringbuf.push_slice(current_line.as_bytes());
                        current_line.clear();
                        if termios.lflag.contains(LFlag::ECHO) {
                            callback(LineControl::Echo(b'\n'));
                        }
                    }
                    ch if termios.is_cooked_mode() => {
                        if 0x20 <= *ch && *ch < 0x7f {
                        // XXX: Should we sleep if the buffer is full?
                        current_line.insert(*ch);
                        if termios.lflag.contains(LFlag::ECHO) {
                            callback(LineControl::Echo(*ch));
                        }
                    }
                    }
                    _ => {
                        // In the raw mode.
                        ringbuf.push(*ch).ok();
                    }
                }

                written_len += 1;
            }
        }

        self.wait_queue.wake_all();
        Ok(written_len)
    }

    pub fn read(&self, dst: UserBufferMut<'_>) -> Result<usize> {
        let mut writer = UserBufWriter::from(dst);
        self.wait_queue.sleep_signalable_until(|| {
            if !self.is_current_foreground() {
                return Ok(None);
            }

            let mut buf_lock = self.buf.lock();
            while writer.remaining_len() > 0 {
                // TODO: Until newline.
                if let Some(slice) = buf_lock.pop_slice(writer.remaining_len()) {
                    writer.write_bytes(slice)?;
                } else {
                    break;
                }
            }

            if writer.written_len() > 0 {
                Ok(Some(writer.written_len()))
            } else {
                Ok(None)
            }
        })
    }
}
