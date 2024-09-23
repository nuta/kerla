use crate::deferred_job::DeferredJob;
use crate::{
    poll::POLL_WAIT_QUEUE, process::WaitQueue, timer::read_monotonic_clock, timer::MonotonicClock,
};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use atomic_refcell::AtomicRefCell;
use crossbeam::queue::ArrayQueue;
use kerla_api::driver::net::EthernetDriver;
use kerla_runtime::bootinfo::BootInfo;
use kerla_runtime::spinlock::SpinLock;
use kerla_utils::once::Once;
use smoltcp::wire::{self, EthernetAddress, IpCidr};
use smoltcp::{
    dhcp::Dhcpv4Client,
    phy::{Device, DeviceCapabilities},
};
use smoltcp::{iface::EthernetInterface, time::Instant};
use smoltcp::{
    iface::{EthernetInterfaceBuilder, NeighborCache, Routes},
    phy::RxToken,
};
use smoltcp::{
    phy::TxToken,
    socket::{RawPacketMetadata, RawSocketBuffer, SocketSet},
    wire::EthernetFrame,
};

pub mod socket;
mod tcp_socket;
mod udp_socket;
mod unix_socket;

pub use socket::*;
pub use tcp_socket::*;
pub use udp_socket::*;
pub use unix_socket::*;

static PACKET_PROCESS_JOB: DeferredJob = DeferredJob::new("net_packet_process");
static RX_PACKET_QUEUE: Once<SpinLock<ArrayQueue<Vec<u8>>>> = Once::new();

pub fn receive_ethernet_frame(frame: &[u8]) {
    if RX_PACKET_QUEUE.lock().push(frame.to_vec()).is_err() {
        // TODO: Introduce warn_once! macro
        warn!("the rx packet queue is full; dropping an incoming packet");
    }

    PACKET_PROCESS_JOB.run_later(|| {
        process_packets();
    });
}

impl From<MonotonicClock> for Instant {
    fn from(value: MonotonicClock) -> Self {
        // FIXME: msecs could be larger than i64
        Instant::from_millis(value.msecs() as i64)
    }
}

static SOCKETS: Once<SpinLock<SocketSet>> = Once::new();
static INTERFACE: Once<SpinLock<EthernetInterface<OurDevice>>> = Once::new();
static DHCP_CLIENT: Once<SpinLock<Dhcpv4Client>> = Once::new();
static DHCP_ENABLED: Once<bool> = Once::new();
static SOCKET_WAIT_QUEUE: Once<WaitQueue> = Once::new();

pub fn process_packets() {
    let mut sockets = SOCKETS.lock();
    let mut iface = INTERFACE.lock();

    let timestamp = read_monotonic_clock().into();
    loop {
        if *DHCP_ENABLED {
            let mut dhcp = DHCP_CLIENT.lock();
            if let Some(config) = dhcp
                .poll(&mut iface, &mut sockets, timestamp)
                .unwrap_or_else(|e| {
                    trace!("DHCP: {:?}", e);
                    None
                })
            {
                if let Some(cidr) = config.address {
                    iface.update_ip_addrs(|addrs| {
                        if let Some(addr) = addrs.iter_mut().next() {
                            *addr = IpCidr::Ipv4(cidr);
                        }
                    });
                    info!("DHCP: got a IPv4 address: {}", cidr);
                }

                config
                    .router
                    .map(|router| iface.routes_mut().add_default_ipv4_route(router).unwrap());
            }
        }

        match iface.poll(&mut sockets, timestamp) {
            Ok(false) => break,
            Ok(true) => {}
            Err(smoltcp::Error::Unrecognized) => {}
            Err(err) => {
                debug_warn!("smoltcp error: {:?}", err);
                break;
            }
        }
    }

    if *DHCP_ENABLED {
        let dhcp = DHCP_CLIENT.lock();
        dhcp.next_poll(timestamp);
    }

    if let Some(_timeout) = iface.poll_delay(&sockets, timestamp) {
        // TODO: Use timeout
    }

    SOCKET_WAIT_QUEUE.wake_all();
    POLL_WAIT_QUEUE.wake_all();
}

struct OurRxToken {
    buffer: Vec<u8>,
}

impl RxToken for OurRxToken {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        f(&mut self.buffer)
    }
}

struct OurTxToken {}

impl TxToken for OurTxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buffer = vec![0; len];
        let return_value = f(&mut buffer)?;
        if EthernetFrame::new_checked(&mut buffer).is_ok() {
            use_ethernet_driver(|driver| driver.transmit(&buffer));
        }

        Ok(return_value)
    }
}

struct OurDevice;

impl<'a> Device<'a> for OurDevice {
    type RxToken = OurRxToken;
    type TxToken = OurTxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        RX_PACKET_QUEUE
            .lock()
            .pop()
            .map(|buffer| (OurRxToken { buffer }, OurTxToken {}))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(OurTxToken {})
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps
    }
}

static ETHERNET_DRIVER: AtomicRefCell<Option<Box<dyn EthernetDriver>>> = AtomicRefCell::new(None);

pub fn register_ethernet_driver(driver: Box<dyn EthernetDriver>) {
    assert!(
        ETHERNET_DRIVER.borrow().is_none(),
        "multiple net drivers are not supported"
    );
    *ETHERNET_DRIVER.borrow_mut() = Some(driver);
}

pub fn use_ethernet_driver<F: FnOnce(&Box<dyn EthernetDriver>) -> R, R>(f: F) -> R {
    let driver = ETHERNET_DRIVER.borrow();
    f(driver.as_ref().expect("no ethernet drivers"))
}

#[derive(Debug)]
struct IPv4AddrParseError;

/// Parses an IPv4 address (e.g. "10.123.123.123").
fn parse_ipv4_addr(addr: &str) -> Result<wire::Ipv4Address, IPv4AddrParseError> {
    let mut iter = addr.splitn(4, '.');
    let mut octets = [0; 4];
    for octet in &mut octets {
        *octet = iter
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or(IPv4AddrParseError)?;
    }

    Ok(wire::Ipv4Address::from_bytes(&octets))
}

/// Parses an IPv4 address with the prefix length (e.g. "10.123.123.123/24").
fn parse_ipv4_addr_with_prefix_len(
    addr: &str,
) -> Result<(wire::Ipv4Address, u8), IPv4AddrParseError> {
    let mut iter = addr.splitn(2, '/');
    let ip = parse_ipv4_addr(iter.next().unwrap())?;
    let prefix_len = iter
        .next()
        .ok_or(IPv4AddrParseError)?
        .parse()
        .map_err(|_| IPv4AddrParseError)?;

    Ok((ip, prefix_len))
}
pub fn init_and_start_dhcp_discover(bootinfo: &BootInfo) {
    let ip_addrs = match &bootinfo.ip4 {
        Some(ip4_str) => {
            let (ip4, prefix_len) = parse_ipv4_addr_with_prefix_len(ip4_str)
                .expect("bootinfo.ip4 should be formed as 10.0.0.1/24");
            info!("net: using a static IPv4 address: {}/{}", ip4, prefix_len);
            [IpCidr::new(ip4.into(), prefix_len)]
        }
        None => [IpCidr::new(wire::Ipv4Address::UNSPECIFIED.into(), 0)],
    };

    let mut routes = Routes::new(BTreeMap::new());
    if let Some(gateway_ip4_str) = &bootinfo.gateway_ip4 {
        let gateway_ip4 = parse_ipv4_addr(gateway_ip4_str)
            .expect("bootinfo.gateway_ip4 should be formed as 10.0.0.1");
        info!("net: using a static gateway IPv4 address: {}", gateway_ip4);
        routes.add_default_ipv4_route(gateway_ip4).unwrap();
    };

    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let mac_addr = use_ethernet_driver(|driver| driver.mac_addr());
    let ethernet_addr = EthernetAddress(mac_addr.as_array());
    let iface = EthernetInterfaceBuilder::new(OurDevice)
        .ethernet_addr(ethernet_addr)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();

    let mut sockets = SocketSet::new(vec![]);

    DHCP_ENABLED.init(|| bootinfo.dhcp_enabled);
    if *DHCP_ENABLED {
        let dhcp_rx_buffer = RawSocketBuffer::new([RawPacketMetadata::EMPTY; 4], vec![0; 2048]);
        let dhcp_tx_buffer = RawSocketBuffer::new([RawPacketMetadata::EMPTY; 4], vec![0; 2048]);
        let dhcp = Dhcpv4Client::new(
            &mut sockets,
            dhcp_rx_buffer,
            dhcp_tx_buffer,
            read_monotonic_clock().into(),
        );
        DHCP_CLIENT.init(|| SpinLock::new(dhcp));
    }
    RX_PACKET_QUEUE.init(|| SpinLock::new(ArrayQueue::new(128)));
    SOCKET_WAIT_QUEUE.init(WaitQueue::new);
    INTERFACE.init(|| SpinLock::new(iface));
    SOCKETS.init(|| SpinLock::new(sockets));

    process_packets();
}
