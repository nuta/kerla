use super::{AF_INET, IPPROTO_TCP, IPPROTO_UDP, SOCK_DGRAM, SOCK_STREAM};
use crate::fs::inode::{FileLike, INode};
use crate::net::{TcpSocket, UdpSocket};
use crate::result::{Errno, Result};
use crate::{process::current_process, syscalls::SyscallDispatcher};
use alloc::sync::Arc;

impl<'a> SyscallDispatcher<'a> {
    pub fn sys_socket(&mut self, domain: i32, type_: i32, protocol: i32) -> Result<isize> {
        // Ignore SOCK_CLOEXEC and SOCK_NONBLOCK for now.
        // FIXME:
        let type_ = type_ & !(0o2000000 | 0o4000);

        let socket = match (domain, type_, protocol) {
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

        let fd = current_process()
            .opened_files
            .lock()
            .open(INode::FileLike(socket))?;

        Ok(fd.as_usize() as isize)
    }
}
