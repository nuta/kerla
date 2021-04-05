use crate::{
    arch::SpinLock,
    fs::inode::{FileLike, PollStatus},
    result::{Errno, Error, Result},
    user_buffer::UserBuffer,
    user_buffer::UserBufferMut,
};
use alloc::{collections::BTreeSet, sync::Arc};
use smoltcp::socket::{UdpPacketMetadata, UdpSocketBuffer};

use super::{process_packets, socket::*, SOCKETS, SOCKET_WAIT_QUEUE};

impl From<Endpoint> for smoltcp::wire::IpEndpoint {
    fn from(endpoint: Endpoint) -> smoltcp::wire::IpEndpoint {
        smoltcp::wire::IpEndpoint {
            port: endpoint.port,
            addr: match endpoint.addr {
                IpAddress::Unspecified => smoltcp::wire::IpAddress::Unspecified,
                IpAddress::Ipv4(addr) => {
                    smoltcp::wire::IpAddress::Ipv4(smoltcp::wire::Ipv4Address::from_bytes(&addr.0))
                }
            },
        }
    }
}

impl From<smoltcp::wire::IpEndpoint> for Endpoint {
    fn from(endpoint: smoltcp::wire::IpEndpoint) -> Endpoint {
        Endpoint {
            port: endpoint.port,
            addr: match endpoint.addr {
                smoltcp::wire::IpAddress::Unspecified => IpAddress::Unspecified,
                smoltcp::wire::IpAddress::Ipv4(addr) => IpAddress::Ipv4(Ipv4Address(addr.0)),
                _ => unreachable!(),
            },
        }
    }
}

static INUSE_ENDPOINTS: SpinLock<BTreeSet<u16>> = SpinLock::new(BTreeSet::new());

pub struct UdpSocket {
    handle: smoltcp::socket::SocketHandle,
}

impl UdpSocket {
    pub fn new() -> Arc<UdpSocket> {
        let rx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY; 64], vec![0; 4096]);
        let tx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY; 64], vec![0; 4096]);
        let inner = smoltcp::socket::UdpSocket::new(rx_buffer, tx_buffer);
        let handle = SOCKETS.lock().add(inner);
        Arc::new(UdpSocket { handle })
    }
}

impl FileLike for UdpSocket {
    fn bind(&self, mut endpoint: Endpoint) -> Result<()> {
        // TODO: Reject if the endpoint is already in use -- IIUC smoltcp
        //       does not check that.
        let mut inuse_endpoints = INUSE_ENDPOINTS.lock();

        if endpoint.port == 0 {
            // Assign a unused port.
            // TODO: Assign a *random* port instead.
            let mut port = 50000;
            while inuse_endpoints.contains(&port) {
                if port == u16::MAX {
                    return Err(errno!(EAGAIN));
                }

                port += 1;
            }
            endpoint.port = port;
        }

        SOCKETS
            .lock()
            .get::<smoltcp::socket::UdpSocket>(self.handle)
            .bind(endpoint)?;
        inuse_endpoints.insert(endpoint.port);

        Ok(())
    }

    fn sendto(&self, mut buf: UserBuffer<'_>, endpoint: Endpoint) -> Result<()> {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<smoltcp::socket::UdpSocket>(self.handle);
        let dst = socket.send(buf.remaining_len(), endpoint.into())?;
        buf.read_bytes(dst)?;

        drop(socket);
        drop(sockets);
        process_packets();
        Ok(())
    }

    fn recvfrom(
        &self,
        mut buf: UserBufferMut<'_>,
        _flags: RecvFromFlags,
    ) -> Result<(usize, Endpoint)> {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<smoltcp::socket::UdpSocket>(self.handle);
        loop {
            match socket.recv() {
                Ok((payload, endpoint)) => {
                    let written_len = buf.write_bytes(payload)?;
                    return Ok((written_len, endpoint.into()));
                }
                Err(smoltcp::Error::Exhausted) if true /* FIXME: if noblock */ => {
                    warn!("recv: EAGAIN");
                    return Err(Error::new(Errno::EAGAIN))
                }
                Err(smoltcp::Error::Exhausted) => {
                    // The receive buffer is empty. Try again later...
                    SOCKET_WAIT_QUEUE.sleep();
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    fn poll(&self) -> Result<PollStatus> {
        let mut sockets = SOCKETS.lock();
        let socket = sockets.get::<smoltcp::socket::UdpSocket>(self.handle);

        let mut status = PollStatus::empty();
        if socket.can_recv() {
            status |= PollStatus::POLLIN;
        }
        if socket.can_send() {
            status |= PollStatus::POLLOUT;
        }

        Ok(status)
    }
}
