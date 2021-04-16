use arrayvec::ArrayVec;
use bitflags::bitflags;

use crate::{
    arch::{print_str, SpinLock},
    fs::{inode::FileLike, opened_file::OpenOptions, stat::Stat},
    process::WaitQueue,
    result::Result,
    user_buffer::UserBuffer,
    user_buffer::UserBufferMut,
};

bitflags! {
    struct LFlag: u32 {
        const ICANON = 0o0000002;
        const ECHO   = 0o0000010;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Termios {
    lflag: LFlag,
}

impl Default for Termios {
    fn default() -> Termios {
        Termios {
            lflag: LFlag::ICANON | LFlag::ECHO,
        }
    }
}

pub struct Tty {
    wait_queue: WaitQueue,
    // TODO: Use an fixed-sized queue which supports iterating all elements.
    buf: SpinLock<ArrayVec<u8, 64>>,
    termios: Termios,
}

impl Tty {
    pub fn new() -> Tty {
        Tty {
            wait_queue: WaitQueue::new(),
            buf: SpinLock::new(ArrayVec::new()),
            termios: Default::default(),
        }
    }

    pub fn input_char(&self, ch: char) {
        if self.termios.lflag.contains(LFlag::ECHO) {
            self.write(0, [ch as u8].as_slice().into(), &OpenOptions::readwrite())
                .ok();
        }

        let mut buf_lock = self.buf.lock();
        match ch as u8 {
            0x7f /* backspace */ if self.is_cooked_mode() => {
                if let Some(b'\n') = buf_lock.pop() {
                    buf_lock.push(b'\n');
                } else {
                    // Remove the previous character by overwriting with a whitespace.
                    print_str(b"\x08 \x08");
                }
            }
            _ => {
                buf_lock.try_push(ch as u8).ok();
            }
        }
        self.wait_queue.wake_one();
    }

    pub fn is_cooked_mode(&self) -> bool {
        self.termios.lflag.contains(LFlag::ICANON)
    }
}

impl FileLike for Tty {
    fn stat(&self) -> Result<Stat> {
        // TODO:
        unimplemented!()
    }

    fn read(
        &self,
        _offset: usize,
        mut dst: UserBufferMut<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        self.wait_queue.sleep_until(|| {
            let mut buf_lock = self.buf.lock();
            if self.is_cooked_mode() {
                if buf_lock.contains(&b'\n') {
                    // Cooked mode: read until the newline.
                    while dst.remaining_len() > 0 {
                        if let Some(ch) = buf_lock.pop_at(0) {
                            dst.write(ch as u8)?;

                            if ch == b'\n' {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            } else {
                while dst.remaining_len() > 0 {
                    if let Some(ch) = buf_lock.pop_at(0) {
                        dst.write(ch as u8)?;
                    } else {
                        break;
                    }
                }
            }

            if dst.pos() > 0 {
                Ok(Some(dst.pos()))
            } else {
                Ok(None)
            }
        })
    }

    fn write(
        &self,
        _offset: usize,
        mut buf: UserBuffer<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        print_str(b"\x1b[1m");
        let mut tmp = [0; 32];
        let mut total_len = 0;
        while buf.remaining_len() > 0 {
            let copied_len = buf.read_bytes(&mut tmp)?;
            print_str(&tmp.as_slice()[..copied_len]);
            total_len += copied_len;
        }
        print_str(b"\x1b[0m");
        Ok(total_len)
    }
}
