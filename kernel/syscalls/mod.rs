use crate::result::{Errno, Error, Result};
use crate::{
    arch::UserVAddr,
    net::{Endpoint, IpAddress, Ipv4Address},
};
use core::mem::size_of;
use penguin_utils::endian::NetworkEndianExt;

pub(self) mod arch_prctl;
pub(self) mod bind;
pub(self) mod brk;
pub(self) mod close;
pub(self) mod dispatcher;
pub(self) mod execve;
pub(self) mod exit;
pub(self) mod fork;
pub(self) mod ioctl;
pub(self) mod open;
pub(self) mod read;
pub(self) mod sendto;
pub(self) mod set_tid_address;
pub(self) mod socket;
pub(self) mod stat;
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
pub const SOCK_DGRAM: i32 = 2;
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
