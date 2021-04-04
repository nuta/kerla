use crate::{
    fs::inode::FileLike,
    result::{Errno, Error, Result},
};
use alloc::sync::Arc;
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

        if endpoint.port == 0 {
            // Assign a random unused port.
            // FIXME:
            endpoint.port = 6767;
        }

        SOCKETS
            .lock()
            .get::<smoltcp::socket::UdpSocket>(self.handle)
            .bind(endpoint)?;
        Ok(())
    }

    fn sendto(&self, buf: &[u8], endpoint: Endpoint) -> Result<()> {
        SOCKETS
            .lock()
            .get::<smoltcp::socket::UdpSocket>(self.handle)
            .send_slice(buf, endpoint.into())?;

        process_packets();
        Ok(())
    }

    fn recvfrom(&self, buf: &mut [u8], _flags: RecvFromFlags) -> Result<(usize, Endpoint)> {
        loop {
            let result = SOCKETS
                .lock()
                .get::<smoltcp::socket::UdpSocket>(self.handle)
                .recv_slice(buf)
                .map(|(len, endpoint)| (len, endpoint.into()));

            match result {
                Ok(result) => {
                    info!("recv: filled {}", result.0);
                    return Ok(result);
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
}
