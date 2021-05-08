use crate::fs::inode::{FileLike, INode};
use crate::net::{socket::*, TcpSocket, UdpSocket, UnixSocket};
use crate::result::{Errno, Result};
use crate::{
    ctypes::*,
    fs::opened_file::{OpenOptions, PathComponent},
};
use crate::{process::current_process, syscalls::SyscallHandler};
use alloc::sync::Arc;
use bitflags::bitflags;

bitflags! {
    struct SocketFlags: c_int {
        const SOCK_NONBLOCK = 0o4000;
        const SOCK_CLOEXEC = 0o2000000;
    }
}

impl From<SocketFlags> for OpenOptions {
    fn from(flags: SocketFlags) -> OpenOptions {
        OpenOptions {
            nonblock: flags.contains(SocketFlags::SOCK_NONBLOCK),
            close_on_exec: flags.contains(SocketFlags::SOCK_CLOEXEC),
        }
    }
}

const SOCKET_TYPE_MASK: c_int = 0xff;

impl<'a> SyscallHandler<'a> {
    pub fn sys_socket(&mut self, domain: i32, type_: i32, protocol: i32) -> Result<isize> {
        let socket_type = type_ & SOCKET_TYPE_MASK;
        let flags = bitflags_from_user!(SocketFlags, type_ & !SOCKET_TYPE_MASK)?;

        let socket = match (domain, socket_type, protocol) {
            (AF_UNIX, SOCK_STREAM, 0) => UnixSocket::new() as Arc<dyn FileLike>,
            (AF_INET, SOCK_DGRAM, 0) | (AF_INET, SOCK_DGRAM, IPPROTO_UDP) => {
                UdpSocket::new() as Arc<dyn FileLike>
            }
            (AF_INET, SOCK_STREAM, 0) | (AF_INET, SOCK_STREAM, IPPROTO_TCP) => {
                TcpSocket::new() as Arc<dyn FileLike>
            }
            (_, _, _) => {
                debug_warn!(
                    "unsupported socket type: domain={}, type={}, protocol={}",
                    domain,
                    type_,
                    protocol
                );
                return Err(Errno::ENOSYS.into());
            }
        };

        let fd = current_process().opened_files().lock().open(
            PathComponent::new_anonymous(INode::FileLike(socket)),
            flags.into(),
        )?;

        Ok(fd.as_usize() as isize)
    }
}
