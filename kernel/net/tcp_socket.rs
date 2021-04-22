use crate::{
    arch::SpinLock,
    fs::{
        inode::{FileLike, PollStatus},
        opened_file::OpenOptions,
    },
    user_buffer::UserBuffer,
    user_buffer::UserBufferMut,
};
use crate::{
    arch::SpinLockGuard,
    result::{Errno, Result},
};
use alloc::{collections::BTreeSet, sync::Arc, vec::Vec};
use core::cmp::min;
use crossbeam::atomic::AtomicCell;
use smoltcp::socket::{SocketRef, TcpSocketBuffer};

use super::{process_packets, socket::*, SOCKETS, SOCKET_WAIT_QUEUE};

const BACKLOG_MAX: usize = 8;
static INUSE_ENDPOINTS: SpinLock<BTreeSet<u16>> = SpinLock::new(BTreeSet::new());

/// Looks for an accept'able socket in the backlog.
fn get_ready_backlog_index(
    sockets: &mut smoltcp::socket::SocketSet,
    backlogs: &[Arc<TcpSocket>],
) -> Option<usize> {
    backlogs.iter().position(|sock| {
        let smol_socket: SocketRef<'_, smoltcp::socket::TcpSocket> = sockets.get(sock.handle);
        smol_socket.may_recv() || smol_socket.may_send()
    })
}

pub struct TcpSocket {
    handle: smoltcp::socket::SocketHandle,
    local_endpoint: AtomicCell<Option<Endpoint>>,
    backlogs: SpinLock<Vec<Arc<TcpSocket>>>,
    num_backlogs: AtomicCell<usize>,
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
            backlogs: SpinLock::new(Vec::new()),
            num_backlogs: AtomicCell::new(0),
        })
    }

    fn refill_backlog_sockets(
        &self,
        backlogs: &mut SpinLockGuard<'_, Vec<Arc<TcpSocket>>>,
    ) -> Result<()> {
        let local_endpoint = match self.local_endpoint.load() {
            Some(local_endpoint) => local_endpoint,
            None => return Err(Errno::EINVAL.into()),
        };

        for _ in 0..(self.num_backlogs.load() - backlogs.len()) {
            let socket = TcpSocket::new();
            SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(socket.handle)
                .listen(local_endpoint)?;
            backlogs.push(socket);
        }

        Ok(())
    }
}

impl FileLike for TcpSocket {
    fn listen(&self, backlog: i32) -> Result<()> {
        let mut backlogs = self.backlogs.lock();

        let new_num_backlogs = min(backlog as usize, BACKLOG_MAX);
        backlogs.truncate(new_num_backlogs);
        self.num_backlogs.store(new_num_backlogs);

        self.refill_backlog_sockets(&mut backlogs)
    }

    fn accept(&self, _options: &OpenOptions) -> Result<(Arc<dyn FileLike>, Endpoint)> {
        SOCKET_WAIT_QUEUE.sleep_until(|| {
            let mut sockets = SOCKETS.lock();
            let mut backlogs = self.backlogs.lock();
            match get_ready_backlog_index(&mut *sockets, &*backlogs) {
                Some(index) => {
                    // Pop the client socket and add a new socket into the backlog.
                    let socket = backlogs.remove(index);
                    drop(sockets);
                    self.refill_backlog_sockets(&mut backlogs)?;

                    // Extract the remote endpoint.
                    let mut sockets_lock = SOCKETS.lock();
                    let smol_socket: SocketRef<'_, smoltcp::socket::TcpSocket> =
                        sockets_lock.get(socket.handle);
                    let endpoint = smol_socket.remote_endpoint().into();
                    Ok(Some((socket as Arc<dyn FileLike>, endpoint)))
                }
                None => {
                    // No accept'able sockets.
                    Ok(None)
                }
            }
        })
    }

    fn bind(&self, endpoint: Endpoint) -> Result<()> {
        // TODO: Reject if the endpoint is already in use -- IIUC smoltcp
        //       does not check that.
        self.local_endpoint.store(Some(endpoint));
        Ok(())
    }

    fn getsockname(&self) -> Result<Endpoint> {
        let endpoint = SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .local_endpoint();

        if endpoint.addr.is_unspecified() {
            return Err(Errno::ENOTCONN.into());
        }

        Ok(endpoint.into())
    }

    fn connect(&self, endpoint: Endpoint, _options: &OpenOptions) -> Result<()> {
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

        // Submit a SYN packet.
        process_packets();

        // Wait until the connection has been established.
        SOCKET_WAIT_QUEUE.sleep_until(|| {
            if SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .may_send()
            {
                Ok(Some(()))
            } else {
                Ok(None)
            }
        })
    }

    fn write(
        &self,
        _offset: usize,
        mut buf: UserBuffer<'_>,
        _options: &OpenOptions,
    ) -> Result<usize> {
        let mut total_len = 0;
        loop {
            let copied_len = SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .send(|dst| {
                    let copied_len = buf.read_bytes(dst).unwrap_or(0);
                    (copied_len, copied_len)
                });

            process_packets();
            match copied_len {
                Ok(0) => {
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

    fn read(
        &self,
        _offset: usize,
        mut buf: UserBufferMut<'_>,
        options: &OpenOptions,
    ) -> Result<usize> {
        SOCKET_WAIT_QUEUE.sleep_until(|| {
            let copied_len = SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .recv(|src| {
                    let copied_len = buf.write_bytes(src).unwrap_or(0);
                    (copied_len, copied_len)
                });

            match copied_len {
                Ok(0) | Err(smoltcp::Error::Exhausted) => {
                    if options.nonblock {
                        Err(Errno::EAGAIN.into())
                    } else {
                        // The receive buffer is empty. Sleep on the wait queue...
                        Ok(None)
                    }
                }
                Ok(copied_len) => {
                    // Continue reading.
                    Ok(Some(copied_len))
                }
                // TODO: Handle FIN
                Err(err) => Err(err.into()),
            }
        })
    }

    fn poll(&self) -> Result<PollStatus> {
        let mut status = PollStatus::empty();
        let mut sockets = SOCKETS.lock();
        if get_ready_backlog_index(&mut *sockets, &*self.backlogs.lock()).is_some() {
            status |= PollStatus::POLLIN;
        }

        let socket = sockets.get::<smoltcp::socket::TcpSocket>(self.handle);
        if socket.can_recv() {
            status |= PollStatus::POLLIN;
        }

        if socket.can_send() {
            status |= PollStatus::POLLOUT;
        }

        Ok(status)
    }
}
