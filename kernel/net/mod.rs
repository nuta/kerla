use crate::{
    arch::SpinLock,
    drivers::{get_ethernet_driver, EthernetDriver},
    process::WaitQueue,
};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crossbeam::queue::ArrayQueue;
use hashbrown::HashMap;
use penguin_utils::once::Once;
use smoltcp::wire::{self, EthernetAddress, IpCidr, Ipv4Cidr};
use smoltcp::{
    dhcp::Dhcpv4Client,
    phy::{Checksum, ChecksumCapabilities, Device, DeviceCapabilities},
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

mod socket;
mod udp_socket;

pub use socket::*;
pub use udp_socket::*;

static RX_PACKET_QUEUE: Once<SpinLock<ArrayQueue<Vec<u8>>>> = Once::new();
static DRIVER: Once<Arc<SpinLock<dyn EthernetDriver>>> = Once::new();

pub fn send_ethernet_frame(frame: &[u8]) {
    DRIVER.lock().transmit(frame).unwrap();
}

pub fn receive_ethernet_frame(frame: &[u8]) {
    if let Err(_) = RX_PACKET_QUEUE.lock().push(frame.to_vec()) {
        // TODO: Introduce warn_once! macro
        warn!("the rx packet queue is full; dropping an incoming packet");
    }

    trace!("received {} bytes", frame.len());
}

pub(self) static SOCKETS: Once<SpinLock<SocketSet>> = Once::new();
static INTERFACE: Once<SpinLock<EthernetInterface<OurDevice>>> = Once::new();
static DHCP_CLIENT: Once<SpinLock<Dhcpv4Client>> = Once::new();
pub(self) static SOCKET_WAIT_QUEUE: WaitQueue = WaitQueue::new();

pub fn iterate_event_loop() {
    let mut sockets = SOCKETS.lock();
    let mut iface = INTERFACE.lock();
    let mut dhcp = DHCP_CLIENT.lock();

    let timestamp = now();
    let mut do_again = true;
    while do_again {
        dhcp.poll(&mut iface, &mut sockets, timestamp)
            .unwrap_or_else(|e| {
                println!("DHCP: {:?}", e);
                None
            })
            .map(|config| {
                info!("DHCP config: {:?}", config);
                if let Some(cidr) = config.address {
                    iface.update_ip_addrs(|addrs| {
                        addrs.iter_mut().next().map(|addr| {
                            *addr = IpCidr::Ipv4(cidr);
                        });
                    });
                    println!("Assigned a new IPv4 address: {}", cidr);
                }

                config
                    .router
                    .map(|router| iface.routes_mut().add_default_ipv4_route(router).unwrap());
                iface.routes_mut().update(|routes_map| {
                    routes_map
                        .get(&IpCidr::new(wire::Ipv4Address::UNSPECIFIED.into(), 0))
                        .map(|default_route| {
                            println!("Default gateway: {}", default_route.via_router);
                        });
                });

                if config.dns_servers.iter().any(|s| s.is_some()) {
                    println!("DNS servers:");
                    for dns_server in config.dns_servers.iter().filter_map(|s| *s) {
                        println!("- {}", dns_server);
                    }
                }
            });

        do_again = match iface.poll(&mut sockets, timestamp) {
            Ok(do_again) => do_again,
            Err(smoltcp::Error::Unrecognized) => true,
            Err(err) => {
                debug_warn!("smoltcp error: {:?}", err);
                false
            }
        };
        if do_again {
            SOCKET_WAIT_QUEUE.wake_all();
        }

        trace!("smotcp: poll, do_again={}", do_again);
    }

    let mut timeout = dhcp.next_poll(timestamp);
    iface
        .poll_delay(&sockets, timestamp)
        .map(|sockets_timeout| timeout = sockets_timeout);
}

pub fn uptime() -> i64 {
    0
}

struct OurRxToken {
    buffer: Vec<u8>,
}

impl RxToken for OurRxToken {
    fn consume<R, F>(mut self, timestamp: Instant, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        f(&mut self.buffer)
    }
}

struct OurTxToken {}

impl TxToken for OurTxToken {
    fn consume<R, F>(self, timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buffer = vec![0; len];
        let return_value = f(&mut buffer)?;
        if let Ok(mut frame) = EthernetFrame::new_checked(&mut buffer) {
            send_ethernet_frame(&buffer);
        }

        Ok(return_value)
    }
}

struct OurDevice;

impl<'a> Device<'a> for OurDevice {
    type RxToken = OurRxToken;
    type TxToken = OurTxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        info!("receive token: {:?}", !RX_PACKET_QUEUE.lock().is_empty());
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

pub fn now() -> Instant {
    Instant::from_millis(uptime())
}

pub fn init() {
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let driver = get_ethernet_driver().expect("no ethernet drivers");
    let mac_addr = driver.lock().mac_addr().unwrap();
    let ethernet_addr = EthernetAddress(mac_addr.as_array());
    let ip_addrs = [IpCidr::new(wire::Ipv4Address::UNSPECIFIED.into(), 0)];
    let routes = Routes::new(BTreeMap::new());
    let mut iface = EthernetInterfaceBuilder::new(OurDevice)
        .ethernet_addr(ethernet_addr)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();

    let mut sockets = SocketSet::new(vec![]);
    let dhcp_rx_buffer = RawSocketBuffer::new([RawPacketMetadata::EMPTY; 4], vec![0; 2048]);
    let dhcp_tx_buffer = RawSocketBuffer::new([RawPacketMetadata::EMPTY; 4], vec![0; 2048]);
    let mut dhcp = Dhcpv4Client::new(&mut sockets, dhcp_rx_buffer, dhcp_tx_buffer, now());

    RX_PACKET_QUEUE.init(|| SpinLock::new(ArrayQueue::new(128)));
    INTERFACE.init(|| SpinLock::new(iface));
    SOCKETS.init(|| SpinLock::new(sockets));
    DHCP_CLIENT.init(|| SpinLock::new(dhcp));
    DRIVER.init(|| driver);

    iterate_event_loop();
}
