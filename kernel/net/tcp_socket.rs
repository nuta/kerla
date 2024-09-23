use crate::{
    fs::{
        inode::{FileLike, PollStatus},
        opened_file::OpenOptions,
    },
    net::{socket::SockAddr, RecvFromFlags},
    result::{Errno, Result},
    user_buffer::UserBuffer,
    user_buffer::{UserBufReader, UserBufWriter, UserBufferMut},
};
use alloc::{collections::BTreeSet, sync::Arc, vec::Vec};
use core::{
    cmp::min,
    convert::TryInto,
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbeam::atomic::AtomicCell;
use kerla_runtime::spinlock::{SpinLock, SpinLockGuard};
use smoltcp::socket::{SocketRef, TcpSocketBuffer};
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};

use super::{process_packets, SOCKETS, SOCKET_WAIT_QUEUE};

const BACKLOG_MAX: usize = 8;
static INUSE_ENDPOINTS: SpinLock<BTreeSet<u16>> = SpinLock::new(BTreeSet::new());
static PASSIVE_OPENS_TOTAL: AtomicUsize = AtomicUsize::new(0);
static WRITTEN_BYTES_TOTAL: AtomicUsize = AtomicUsize::new(0);
static READ_BYTES_TOTAL: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub struct Stats {
    pub passive_opens_total: usize,
    pub written_bytes_total: usize,
    pub read_bytes_total: usize,
}

pub fn read_tcp_stats() -> Stats {
    Stats {
        passive_opens_total: PASSIVE_OPENS_TOTAL.load(Ordering::SeqCst),
        written_bytes_total: WRITTEN_BYTES_TOTAL.load(Ordering::SeqCst),
        read_bytes_total: READ_BYTES_TOTAL.load(Ordering::SeqCst),
    }
}

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
    local_endpoint: AtomicCell<Option<IpEndpoint>>,
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

    fn accept(&self, _options: &OpenOptions) -> Result<(Arc<dyn FileLike>, SockAddr)> {
        SOCKET_WAIT_QUEUE.sleep_signalable_until(|| {
            let mut sockets = SOCKETS.lock();
            let mut backlogs = self.backlogs.lock();
            match get_ready_backlog_index(&mut sockets, &backlogs) {
                Some(index) => {
                    // Pop the client socket and add a new socket into the backlog.
                    let socket = backlogs.remove(index);
                    drop(sockets);
                    self.refill_backlog_sockets(&mut backlogs)?;

                    // Extract the remote endpoint.
                    let mut sockets_lock = SOCKETS.lock();
                    let smol_socket: SocketRef<'_, smoltcp::socket::TcpSocket> =
                        sockets_lock.get(socket.handle);

                    PASSIVE_OPENS_TOTAL.fetch_add(1, Ordering::SeqCst);

                    Ok(Some((
                        socket as Arc<dyn FileLike>,
                        smol_socket.remote_endpoint().into(),
                    )))
                }
                None => {
                    // No accept'able sockets.
                    Ok(None)
                }
            }
        })
    }

    fn bind(&self, sockaddr: SockAddr) -> Result<()> {
        // TODO: Reject if the endpoint is already in use -- IIUC smoltcp
        //       does not check that.
        self.local_endpoint.store(Some(sockaddr.try_into()?));
        Ok(())
    }

    fn shutdown(&self, _how: super::ShutdownHow) -> Result<()> {
        SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .close();

        process_packets();
        Ok(())
    }

    fn getsockname(&self) -> Result<SockAddr> {
        let endpoint = SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .local_endpoint();

        if endpoint.addr.is_unspecified() {
            return Err(Errno::ENOTCONN.into());
        }

        Ok(endpoint.into())
    }

    fn getpeername(&self) -> Result<SockAddr> {
        let endpoint = SOCKETS
            .lock()
            .get::<smoltcp::socket::TcpSocket>(self.handle)
            .remote_endpoint();

        if endpoint.addr.is_unspecified() {
            return Err(Errno::ENOTCONN.into());
        }

        Ok(endpoint.into())
    }

    fn connect(&self, sockaddr: SockAddr, _options: &OpenOptions) -> Result<()> {
        let remote_endpoint: IpEndpoint = sockaddr.try_into()?;

        // TODO: Reject if the endpoint is already in use -- IIUC smoltcp
        //       does not check that.
        let mut inuse_endpoints = INUSE_ENDPOINTS.lock();
        let mut local_endpoint = self.local_endpoint.load().unwrap_or(IpEndpoint {
            addr: IpAddress::Ipv4(Ipv4Address::UNSPECIFIED),
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
            .connect(remote_endpoint, local_endpoint)?;
        inuse_endpoints.insert(remote_endpoint.port);
        drop(inuse_endpoints);

        // Submit a SYN packet.
        process_packets();

        // Wait until the connection has been established.
        SOCKET_WAIT_QUEUE.sleep_signalable_until(|| {
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

    fn write(&self, _offset: usize, buf: UserBuffer<'_>, _options: &OpenOptions) -> Result<usize> {
        let mut total_len = 0;
        let mut reader = UserBufReader::from(buf);
        loop {
            let copied_len = SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .send(|dst| {
                    let copied_len = reader.read_bytes(dst).unwrap_or(0);
                    (copied_len, copied_len)
                });

            process_packets();
            match copied_len {
                Ok(0) => {
                    WRITTEN_BYTES_TOTAL.fetch_add(total_len, Ordering::SeqCst);
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

    fn read(&self, _offset: usize, buf: UserBufferMut<'_>, options: &OpenOptions) -> Result<usize> {
        let mut writer = UserBufWriter::from(buf);
        SOCKET_WAIT_QUEUE.sleep_signalable_until(|| {
            let copied_len = SOCKETS
                .lock()
                .get::<smoltcp::socket::TcpSocket>(self.handle)
                .recv(|src| {
                    let copied_len = writer.write_bytes(src).unwrap_or(0);
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
                    READ_BYTES_TOTAL.fetch_add(copied_len, Ordering::SeqCst);
                    Ok(Some(copied_len))
                }
                // TODO: Handle FIN
                Err(err) => Err(err.into()),
            }
        })
    }

    fn sendto(
        &self,
        buf: UserBuffer<'_>,
        sockaddr: Option<SockAddr>,
        options: &OpenOptions,
    ) -> Result<usize> {
        if sockaddr.is_some() {
            return Err(Errno::EINVAL.into());
        }

        self.write(0, buf, options)
    }

    fn recvfrom(
        &self,
        buf: UserBufferMut<'_>,
        _flags: RecvFromFlags,
        options: &OpenOptions,
    ) -> Result<(usize, SockAddr)> {
        Ok((self.read(0, buf, options)?, self.getpeername()?))
    }

    fn poll(&self) -> Result<PollStatus> {
        let mut status = PollStatus::empty();
        let mut sockets = SOCKETS.lock();
        if get_ready_backlog_index(&mut sockets, &self.backlogs.lock()).is_some() {
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

impl Drop for TcpSocket {
    fn drop(&mut self) {
        SOCKETS.lock().remove(self.handle);
    }
}

impl fmt::Debug for TcpSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TcpSocket").finish()
    }
}
