use alloc::sync::Arc;
use penguin_utils::{once::Once, ring_buffer::RingBuffer};

use crate::{
    arch::SpinLock,
    fs::{
        inode::{FileLike, PollStatus},
        opened_file::OpenOptions,
    },
    process::WaitQueue,
    result::{Errno, Result},
    user_buffer::{UserBuffer, UserBufferMut},
};

const PIPE_SIZE: usize = 4096;

// TODO: Fine-granined wait queue, say, embed a queue in every pipes.
static PIPE_WAIT_QUEUE: Once<WaitQueue> = Once::new();

struct PipeInner {
    buf: RingBuffer<u8, PIPE_SIZE>,
}

pub struct Pipe(Arc<SpinLock<PipeInner>>);

impl Pipe {
    pub fn new() -> Pipe {
        Pipe(Arc::new(SpinLock::new(PipeInner {
            buf: RingBuffer::new(),
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
    fn write(
        &self,
        _offset: usize,
        mut buf: UserBuffer<'_>,
        options: &OpenOptions,
    ) -> Result<usize> {
        // TODO: Implement EPIPE and SIGPIPE

        let ret_value = PIPE_WAIT_QUEUE.sleep_until(|| {
            let mut written_len = 0;
            loop {
                let mut tmp = [0; 512];
                let copied_len = buf.read_bytes(&mut tmp)?;
                if copied_len == 0 {
                    break;
                }

                match self.0.lock().buf.push_slice(&tmp[..copied_len]) {
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

pub struct PipeReader(Arc<SpinLock<PipeInner>>);

impl FileLike for PipeReader {
    fn write(&self, _offset: usize, _buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        Err(Errno::EINVAL.into())
    }

    fn read(
        &self,
        _offset: usize,
        mut buf: UserBufferMut<'_>,
        options: &OpenOptions,
    ) -> Result<usize> {
        // TODO: Return Ok(0) if there're no writers.

        let ret_value = PIPE_WAIT_QUEUE.sleep_until(|| {
            let mut read_len = 0;
            while let Some(src) = self.0.lock().buf.pop_slice(buf.remaining_len()) {
                read_len += buf.write_bytes(src)?;
            }

            if read_len > 0 {
                Ok(Some(read_len))
            } else if options.nonblock {
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

pub fn init() {
    PIPE_WAIT_QUEUE.init(|| WaitQueue::new());
}
