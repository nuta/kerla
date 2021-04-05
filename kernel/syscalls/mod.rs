use crate::result::{Errno, Error, Result};
use crate::{
    arch::UserVAddr,
    net::{Endpoint, IpAddress, Ipv4Address},
};
use core::cmp::min;
use core::mem::size_of;
use penguin_utils::{alignment::align_up, endian::NetworkEndianExt};

macro_rules! bitflags_from_user {
    ($st:tt, $input:expr) => {{
        let bits = $input;
        $st::from_bits(bits).ok_or_else(|| {
            warn_once!(
                concat!("unsupported bitflags for ", stringify!($st), ": {:x}"),
                bits
            );

            crate::result::Error::new(crate::result::Errno::ENOSYS)
        })
    }};
}

pub(self) mod arch_prctl;
pub(self) mod bind;
pub(self) mod brk;
pub(self) mod chdir;
pub(self) mod clock_gettime;
pub(self) mod close;
pub(self) mod connect;
pub(self) mod dispatcher;
pub(self) mod execve;
pub(self) mod exit;
pub(self) mod fork;
pub(self) mod getdents64;
pub(self) mod getrandom;
pub(self) mod ioctl;
pub(self) mod lstat;
pub(self) mod mkdir;
pub(self) mod open;
pub(self) mod poll;
pub(self) mod read;
pub(self) mod recvfrom;
pub(self) mod sendto;
pub(self) mod set_tid_address;
pub(self) mod socket;
pub(self) mod stat;
pub(self) mod uname;
pub(self) mod utimes;
pub(self) mod wait4;
pub(self) mod write;
pub(self) mod writev;

pub use dispatcher::SyscallDispatcher;

pub(self) const MAX_READ_WRITE_LEN: usize = core::isize::MAX as usize;
pub(self) const IOV_MAX: usize = 1024;

#[repr(C)]
pub(self) struct IoVec {
    base: UserVAddr,
    len: usize,
}

pub const AF_INET: i32 = 2;
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM: i32 = 2;
pub const IPPROTO_TCP: i32 = 6;
pub const IPPROTO_UDP: i32 = 17;

#[allow(non_camel_case_types)]
pub type sa_family_t = u16;
#[allow(non_camel_case_types)]
pub type socklen_t = u32;

#[non_exhaustive]
pub enum SockAddr {
    In(SockAddrIn),
}

impl From<SockAddr> for Endpoint {
    fn from(sockaddr: SockAddr) -> Self {
        match sockaddr {
            SockAddr::In(sockaddr_in) => Endpoint {
                addr: IpAddress::Ipv4(Ipv4Address::from(sockaddr_in.addr)),
                port: sockaddr_in.port,
            },
        }
    }
}

/// `struct sockaddr_in`
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct SockAddrIn {
    /// `AF_INET`
    family: sa_family_t,
    /// The port number in host's byte order.
    port: u16,
    /// The IPv4 address in host's byte order.
    addr: u32,
    /// Unused padding area.
    zero: [u8; 8],
}

pub(self) fn parse_sockaddr(uaddr: UserVAddr, _len: usize) -> Result<SockAddr> {
    // TODO: Check `len`
    let sa_family = uaddr.read::<sa_family_t>()?;
    let sockaddr = match sa_family as i32 {
        AF_INET => {
            let mut sockaddr_in = uaddr.read::<SockAddrIn>()?;
            sockaddr_in.port = sockaddr_in.port.from_network_endian();
            sockaddr_in.addr = sockaddr_in.addr.from_network_endian();
            SockAddr::In(sockaddr_in)
        }
        _ => {
            // FIXME: Is EINVAL correct error code?
            return Err(Error::new(Errno::EINVAL));
        }
    };

    Ok(sockaddr)
}

pub fn write_endpoint_as_sockaddr(
    endpoint: &Endpoint,
    sockaddr: UserVAddr,
    socklen: UserVAddr,
) -> Result<()> {
    match endpoint.addr {
        IpAddress::Ipv4(addr) => {
            if !sockaddr.is_null() {
                let mut offset = 0;
                // family
                info!(
                    "write: {:?} {}, off={}, port={:02x?} {:02x?} {:02x?}",
                    addr,
                    endpoint.port,
                    offset,
                    &endpoint.port.to_le_bytes(),
                    &endpoint.port.to_be_bytes(),
                    &endpoint.port.to_ne_bytes()
                );
                offset += sockaddr
                    .add(offset)?
                    .write::<sa_family_t>(&(AF_INET as sa_family_t))?;
                // port
                info!("write: {:?} {}, off={}", addr, endpoint.port, offset);
                offset += sockaddr
                    .add(offset)?
                    .write_bytes(&endpoint.port.to_be_bytes())?;
                // addr
                info!("write: {:?} {}, off={}", addr, endpoint.port, offset);
                offset += sockaddr.add(offset)?.write_bytes(&addr.0)?;
                // zero
                sockaddr.add(offset)?.write_bytes(&[0; 8])?;

                let mut wrr = vec![0; 16];
                sockaddr.read_bytes(&mut wrr).unwrap();
                info!("sockaddr_r = {:02x?}", wrr);
            }

            if !socklen.is_null() {
                socklen.write::<socklen_t>(&(size_of::<SockAddrIn>() as u32))?;
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

pub struct UserBufReader {
    base: UserVAddr,
    pos: usize,
}

impl UserBufReader {
    pub const fn new(base: UserVAddr) -> UserBufReader {
        UserBufReader { base, pos: 0 }
    }

    pub fn pos(&mut self) -> usize {
        self.pos
    }

    pub fn skip(&mut self, len: usize) {
        self.pos += len;
    }

    pub fn read<T: Copy>(&mut self) -> Result<T> {
        let value = self.base.add(self.pos)?.read()?;
        self.pos += size_of::<T>();
        Ok(value)
    }
}

pub struct UserBufWriter {
    base: UserVAddr,
    pos: usize,
}

impl UserBufWriter {
    pub const fn new(base: UserVAddr) -> UserBufWriter {
        UserBufWriter { base, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn skip_until_alignment(&mut self, align: usize) {
        self.pos = align_up(self.pos, align);
    }

    pub fn fill(&mut self, value: u8, len: usize) -> Result<()> {
        self.pos += self.base.add(self.pos)?.fill(value, len)?;
        Ok(())
    }

    pub fn write<T: Copy>(&mut self, value: T) -> Result<()> {
        let written_len = self.base.add(self.pos)?.write(&value)?;
        self.pos += written_len;
        Ok(())
    }

    pub fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        let written_len = self.base.add(self.pos)?.write_bytes(buf)?;
        self.pos += written_len;
        Ok(())
    }

    pub fn write_bytes_or_zeroes(&mut self, buf: &[u8], max_copy_len: usize) -> Result<()> {
        let zeroed_after = min(buf.len(), max_copy_len);
        self.write_bytes(&buf[..zeroed_after])?;
        self.fill(0, max_copy_len - zeroed_after)?;
        Ok(())
    }
}
