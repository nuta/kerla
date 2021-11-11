use crate::{
    fs::{
        inode::{FileLike, PollStatus},
        opened_file::OpenOptions,
    },
    result::{Errno, Error, Result},
    user_buffer::UserBuffer,
    user_buffer::{UserBufReader, UserBufWriter, UserBufferMut},
};
use alloc::{collections::BTreeSet, sync::Arc};
use core::{convert::TryInto, fmt};
use kerla_runtime::spinlock::SpinLock;
use smoltcp::socket::{UdpPacketMetadata, UdpSocketBuffer};
use smoltcp::wire::IpEndpoint;

use super::{process_packets, socket::*, SOCKETS, SOCKET_WAIT_QUEUE};

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
    fn bind(&self, sockaddr: SockAddr) -> Result<()> {
        let mut endpoint: IpEndpoint = sockaddr.try_into()?;
        // TODO: Reject if the endpoint is already in use -- IIUC smoltcp
        //       does not check that.
        let mut inuse_endpoints = INUSE_ENDPOINTS.lock();

        if endpoint.port == 0 {
            // Assign a unused port.
            // TODO: Assign a *random* port instead.
            let mut port = 50000;
            while inuse_endpoints.contains(&port) {
                if port == u16::MAX {
                    return Err(Errno::EAGAIN.into());
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

    fn sendto(
        &self,
        buf: UserBuffer<'_>,
        sockaddr: Option<SockAddr>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        let endpoint: IpEndpoint = sockaddr
            .ok_or_else(|| Error::new(Errno::EINVAL))?
            .try_into()?;
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<smoltcp::socket::UdpSocket>(self.handle);
        let mut reader = UserBufReader::from(buf);
        let dst = socket.send(reader.remaining_len(), endpoint)?;
        let copied_len = reader.read_bytes(dst)?;

        drop(socket);
        drop(sockets);
        process_packets();
        Ok(copied_len)
    }

    fn recvfrom(
        &self,
        buf: UserBufferMut<'_>,
        _flags: RecvFromFlags,
        options: &OpenOptions,
    ) -> Result<(usize, SockAddr)> {
        let mut writer = UserBufWriter::from(buf);
        SOCKET_WAIT_QUEUE.sleep_signalable_until(|| {
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<smoltcp::socket::UdpSocket>(self.handle);
            match socket.recv() {
                Ok((payload, endpoint)) => {
                    writer.write_bytes(payload)?;
                    Ok(Some((writer.written_len(), endpoint.into())))
                }
                Err(smoltcp::Error::Exhausted) if options.nonblock => Err(Errno::EAGAIN.into()),
                Err(smoltcp::Error::Exhausted) => {
                    // The receive buffer is empty. Try again later...
                    Ok(None)
                }
                Err(err) => Err(err.into()),
            }
        })
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

impl fmt::Debug for UdpSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UdpSocket").finish()
    }
}
