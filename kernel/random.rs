use crate::result::Result;
use crate::user_buffer::UserBufferMut;
use x86::random::rdrand_slice;

pub fn read_secure_random(mut buf: UserBufferMut<'_>) -> Result<usize> {
    buf.write_with(|slice| {
        let valid = unsafe { rdrand_slice(slice) };
        if valid {
            Ok(slice.len())
        } else {
            warn_once!("RDRAND returned invalid data");
            Ok(0)
        }
    })
}

pub fn read_insecure_random(buf: UserBufferMut<'_>) -> Result<usize> {
    // TODO:
    read_secure_random(buf)
}
