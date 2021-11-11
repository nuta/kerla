//! Unnamed pipe (`pipe(2)`).
use core::fmt;

use kerla_runtime::spinlock::SpinLock;
use kerla_utils::{once::Once, ring_buffer::RingBuffer};

use crate::{
    fs::{
        inode::{FileLike, PollStatus},
        opened_file::OpenOptions,
    },
    prelude::*,
    process::WaitQueue,
    user_buffer::{UserBufReader, UserBufWriter, UserBuffer, UserBufferMut},
};

const PIPE_SIZE: usize = 4096;

// TODO: Fine-granined wait queue, say, embed a queue in every pipes.
static PIPE_WAIT_QUEUE: Once<WaitQueue> = Once::new();

struct PipeInner {
    buf: RingBuffer<u8, PIPE_SIZE>,
    closed_by_reader: bool,
    closed_by_writer: bool,
}

pub struct Pipe(Arc<SpinLock<PipeInner>>);

impl Pipe {
    pub fn new() -> Pipe {
        Pipe(Arc::new(SpinLock::new(PipeInner {
            buf: RingBuffer::new(),
            closed_by_reader: false,
            closed_by_writer: false,
        })))
    }

    pub fn write_end(&self) -> Arc<PipeWriter> {
        Arc::new(PipeWriter(self.0.clone()))
    }

    pub fn read_end(&self) -> Arc<PipeReader> {
        Arc::new(PipeReader(self.0.clone()))
    }
}

pub struct PipeWriter(Arc<SpinLock<PipeInner>>);

impl FileLike for PipeWriter {
    fn write(&self, _offset: usize, buf: UserBuffer<'_>, options: &OpenOptions) -> Result<usize> {
        let ret_value = PIPE_WAIT_QUEUE.sleep_signalable_until(|| {
            let mut pipe = self.0.lock();
            if pipe.closed_by_reader {
                // TODO: SIGPIPE?
                return Err(Errno::EPIPE.into());
            }

            let mut written_len = 0;
            let mut reader = UserBufReader::from(buf.clone());
            loop {
                let mut tmp = [0; 512];
                let copied_len = reader.read_bytes(&mut tmp)?;
                if copied_len == 0 {
                    break;
                }

                match pipe.buf.push_slice(&tmp[..copied_len]) {
                    0 => break,
                    len => {
                        written_len += len;
                    }
                }
            }

            if written_len > 0 {
                Ok(Some(written_len))
            } else if options.nonblock {
                Ok(Some(0))
            } else {
                Ok(None)
            }
        });

        // Try waking readers...
        PIPE_WAIT_QUEUE.wake_all();
        ret_value
    }

    fn read(
        &self,
        _offset: usize,
        _buf: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        Err(Errno::EINVAL.into())
    }

    fn poll(&self) -> Result<PollStatus> {
        let mut status = PollStatus::empty();
        let inner = self.0.lock();

        if inner.buf.is_writable() {
            status |= PollStatus::POLLOUT;
        }

        Ok(status)
    }
}

impl fmt::Debug for PipeWriter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipeWriter").finish()
    }
}

impl Drop for PipeWriter {
    fn drop(&mut self) {
        self.0.lock().closed_by_writer = true;
        PIPE_WAIT_QUEUE.wake_all();
    }
}

pub struct PipeReader(Arc<SpinLock<PipeInner>>);

impl FileLike for PipeReader {
    fn write(&self, _offset: usize, _buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        Err(Errno::EINVAL.into())
    }

    fn read(&self, _offset: usize, buf: UserBufferMut<'_>, options: &OpenOptions) -> Result<usize> {
        let mut writer = UserBufWriter::from(buf);
        let ret_value = PIPE_WAIT_QUEUE.sleep_signalable_until(|| {
            let mut pipe = self.0.lock();

            while let Some(src) = pipe.buf.pop_slice(writer.remaining_len()) {
                writer.write_bytes(src)?;
            }

            if writer.written_len() > 0 {
                Ok(Some(writer.written_len()))
            } else if options.nonblock || pipe.closed_by_writer {
                Ok(Some(0))
            } else {
                Ok(None)
            }
        });

        // Try waking writers...
        PIPE_WAIT_QUEUE.wake_all();
        ret_value
    }

    fn poll(&self) -> Result<PollStatus> {
        let mut status = PollStatus::empty();
        let inner = self.0.lock();

        if inner.buf.is_readable() {
            status |= PollStatus::POLLIN;
        }

        Ok(status)
    }
}

impl fmt::Debug for PipeReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PipeReader").finish()
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        self.0.lock().closed_by_reader = false;
        PIPE_WAIT_QUEUE.wake_all();
    }
}

pub fn init() {
    PIPE_WAIT_QUEUE.init(WaitQueue::new);
}
