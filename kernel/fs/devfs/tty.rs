use crate::{
    arch::print_str,
    fs::{
        inode::{FileLike, INodeNo},
        opened_file::OpenOptions,
        stat::{FileMode, Stat, S_IFCHR},
    },
    result::Result,
    tty::line_discipline::*,
    user_buffer::UserBuffer,
    user_buffer::UserBufferMut,
};

pub struct Tty {
    discipline: LineDiscipline,
}

impl Tty {
    pub fn new() -> Tty {
        Tty {
            discipline: LineDiscipline::new(),
        }
    }

    pub fn input_char(&self, ch: u8) {
        self.discipline
            .write(([ch].as_slice()).into(), |ctrl| {
                match ctrl {
                    LineControl::Backspace => {
                        // Remove the previous character by overwriting with a whitespace.
                        print_str(b"\x08 \x08");
                    }
                    LineControl::Echo(ch) => {
                        self.write(0, [ch].as_slice().into(), &OpenOptions::readwrite())
                            .ok();
                    }
                }
            })
            .ok();
    }
}

impl FileLike for Tty {
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
