use crate::result::{Errno, Result};
use crate::{
    arch::SpinLock, fs::inode::FileLike, user_buffer::UserBuffer, user_buffer::UserBufferMut,
};
use alloc::{collections::BTreeSet, sync::Arc};
use crossbeam::atomic::AtomicCell;
use smoltcp::socket::TcpSocketBuffer;

use super::{process_packets, socket::*, SOCKETS, SOCKET_WAIT_QUEUE};

static INUSE_ENDPOINTS: SpinLock<BTreeSet<u16>> = SpinLock::new(BTreeSet::new());

pub struct TcpSocket {
    handle: smoltcp::socket::SocketHandle,
    local_endpoint: AtomicCell<Option<Endpoint>>,
}

impl TcpSocket {
    pub fn new() -> Arc<TcpSocket> {
        let rx_buffer = TcpSocketBuffer::new(vec![0; 4096]);
        let tx_buffer = TcpSocketBuffer::new(vec![0; 4096]);
        let inner = smoltcp::socket::TcpSocket::new(rx_buffer, tx_buffer);
        let handle = SOCKETS.lock().add(inner);
        Arc::new(TcpSocket {
            handle,
            local_endpoint: AtomicCell::new(None),
        })
    }
}

impl FileLike for TcpSocket {
    fn bind(&self, endpoint: Endpoint) -> Result<()> {
        // TODO: Reject if the endpoint is already in use -- IIUC smoltcp
        //       does not check that.
        self.local_endpoint.store(Some(endpoint));
        Ok(())
    }

    fn connect(&self, endpoint: Endpoint) -> Result<()> {
        // TODO: Reject if the endpoint is already in use -- IIUC smoltcp
        //       does not check that.
        let mut inuse_endpoints = INUSE_ENDPOINTS.lock();

        let mut local_endpoint = self.local_endpoint.load().unwrap_or(Endpoint {
            addr: IpAddress::Unspecified,
            port: 0,
        });
        if local_endpoint.port == 0 {
            // Assign a unused port.
            // TODO: Assign a *random* port instead.
            let mut port = 50000;
            while inuse_endpoints.contains(&port) {
                if port == u16::MAX {
                    return Err(Errno::EAGAIN.into());
                }

                port += 1;
            }
            local_endpoint.port = port;
        }

        SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .connect(endpoint, local_endpoint)?;
        inuse_endpoints.insert(endpoint.port);
        drop(inuse_endpoints);

        process_packets();
        while !SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .may_send()
        {
            SOCKET_WAIT_QUEUE.sleep();
        }

        Ok(())
    }

    fn write(&self, _offset: usize, mut buf: UserBuffer<'_>) -> Result<usize> {
        let mut total_len = 0;
        loop {
            let copied_len = SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .send(|dst| {
                    let copied_len = buf.read_bytes(dst).unwrap_or(0);
                    (copied_len, copied_len)
                });

            match copied_len {
                Ok(0) => {
                    process_packets();
                    return Ok(total_len);
                }
                Ok(copied_len) => {
                    // Continue writing.
                    total_len += copied_len;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    fn read(&self, _offset: usize, mut buf: UserBufferMut<'_>) -> Result<usize> {
        let mut total_len = 0;
        loop {
            let copied_len = SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .recv(|src| {
                    let copied_len = buf.write_bytes(src).unwrap_or(0);
                    (copied_len, copied_len)
                });

            match copied_len {
                Ok(0) => {
                    return Ok(total_len);
                }
                Ok(copied_len) => {
                    // Continue reading.
                    total_len += copied_len;
                }
                Err(smoltcp::Error::Exhausted) if true /* FIXME: if noblock */ => {
                    return Err(Errno::EAGAIN.into())
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
