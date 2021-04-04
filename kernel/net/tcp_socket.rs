use crate::{
    fs::inode::FileLike,
    result::{Errno, Error, Result},
};
use alloc::sync::Arc;
use crossbeam::atomic::AtomicCell;
use smoltcp::socket::TcpSocketBuffer;

use super::{process_packets, socket::*, SOCKETS, SOCKET_WAIT_QUEUE};

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
        let mut local_endpoint = self.local_endpoint.load().unwrap_or(Endpoint {
            addr: IpAddress::Unspecified,
            port: 0,
        });

        if local_endpoint.port == 0 {
            // Assign a random unused port.
            // FIXME:
            local_endpoint.port = 6768;
        }

        SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .connect(endpoint, local_endpoint)?;

        process_packets();
        while !SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .may_send()
        {
            trace!("tcp: wait queue");
            SOCKET_WAIT_QUEUE.sleep();
        }

        Ok(())
    }

    fn write(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        let written_len = SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .send_slice(buf)?;

        process_packets();
        Ok(written_len)
    }

    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        loop {
            let result = SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .recv_slice(buf);

            match result {
                Ok(read_len) => {
                    info!("tcp: read {}", read_len);
                    return Ok(read_len);
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
