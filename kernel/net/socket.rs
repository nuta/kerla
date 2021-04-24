use crate::{arch::UserVAddr, result::*};
use bitflags::bitflags;
use core::convert::TryFrom;
use core::mem::size_of;
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};

bitflags! {
    pub struct RecvFromFlags: i32 {
        // TODO:
        const _NOT_IMPLEMENTED = 0;
    }
}

bitflags! {
    pub struct SendToFlags: i32 {
        // TODO:
        const _NOT_IMPLEMENTED = 0;
        const MSG_NOSIGNAL = 0x4000;
    }
}

pub const AF_UNIX: i32 = 1;
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
#[derive(Debug, Clone)]
pub enum SockAddr {
    In(SockAddrIn),
}

/// `struct sockaddr_in`
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct SockAddrIn {
    /// `AF_INET`
    family: sa_family_t,
    /// The port number in the network byte order.
    port: [u8; 2],
    /// The IPv4 address in the network byte order.
    addr: [u8; 4],
    /// Unused padding area.
    zero: [u8; 8],
}

impl TryFrom<SockAddr> for IpEndpoint {
    type Error = Error;
    fn try_from(sockaddr: SockAddr) -> Result<IpEndpoint> {
        match sockaddr {
            SockAddr::In(SockAddrIn { port, addr, .. }) => Ok(IpEndpoint {
                port: u16::from_be_bytes(port),
                addr: if u32::from_be_bytes(addr) == 0 {
                    IpAddress::Unspecified
                } else {
                    IpAddress::Ipv4(smoltcp::wire::Ipv4Address(addr))
                },
            }),
        }
    }
}

impl From<IpEndpoint> for SockAddr {
    fn from(endpoint: IpEndpoint) -> SockAddr {
        SockAddr::In(SockAddrIn {
            family: AF_INET as u16,
            port: endpoint.port.to_be_bytes(),
            addr: match endpoint.addr {
                IpAddress::Unspecified => Ipv4Address::UNSPECIFIED.0,
                IpAddress::Ipv4(addr) => addr.0,
                _ => unimplemented!(),
            },
            zero: [0; 8],
        })
    }
}

pub fn parse_sockaddr(uaddr: UserVAddr, _len: usize) -> Result<SockAddr> {
    // TODO: Check `len`
    let sa_family = uaddr.read::<sa_family_t>()?;
    let sockaddr = match sa_family as i32 {
        AF_INET => SockAddr::In(uaddr.read::<SockAddrIn>()?),
        AF_UNIX => {
            // let offset = size_of::<sa_family_t>();
            // let path = UserCStr::new(uaddr.add(offset)?, len.saturating_sub(offset))?;
            // SockAddr::Unix(PathBuf::from(path.as_str()?))
            // FIXME:
            return Err(Errno::EACCES.into());
        }
        _ => {
            // FIXME: Is EINVAL correct error code?
            return Err(Errno::EINVAL.into());
        }
    };

    Ok(sockaddr)
}

pub fn write_endpoint_as_sockaddr(
    sockaddr: &SockAddr,
    dst: UserVAddr,
    socklen: UserVAddr,
) -> Result<()> {
    match sockaddr {
        SockAddr::In(sockaddr_in) => {
            if !dst.is_null() {
                dst.write::<SockAddrIn>(sockaddr_in)?;
            }

            if !socklen.is_null() {
                socklen.write::<socklen_t>(&(size_of::<SockAddrIn>() as u32))?;
            }
        }
    }

    Ok(())
}
