//! A virtio-net device driver.
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::mem::size_of;
use kerla_api::driver::register_driver_builder;
use kerla_api::net::receive_ethernet_frame;
use memoffset::offset_of;

use crate::transports::virtio_pci::VirtioAttachError;
use crate::transports::{virtio_mmio::VirtioMmio, virtio_pci::VirtioPci, VirtioTransport};
use crate::virtio::{IsrStatus, Virtio, VirtqUsedChain};

use kerla_api::address::VAddr;
use kerla_api::arch::PAGE_SIZE;
use kerla_api::driver::{
    attach_irq,
    net::{register_ethernet_driver, Driver, EthernetDriver, MacAddress},
    DriverBuilder,
};
use kerla_api::driver::{pci::PciDevice, VirtioMmioDevice};
use kerla_api::mm::{alloc_pages, AllocPageFlags};
use kerla_api::sync::SpinLock;
use kerla_utils::alignment::align_up;

const VIRTIO_NET_F_MAC: u64 = 1 << 5;

const VIRTIO_NET_QUEUE_RX: u16 = 0;
const VIRTIO_NET_QUEUE_TX: u16 = 1;

const PACKET_LEN_MAX: usize = 2048;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtioNetHeader {
    flags: u8,
    gso_type: u8,
    hdr_len: u16,
    gso_size: u16,
    checksum_start: u16,
    checksum_offset: u16,
    num_buffer: u16,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtioNetConfig {
    mac: [u8; 6],
    status: u16,
    max_virtqueue_pairs: u16,
    mtu: u16,
}

pub struct VirtioNet {
    mac_addr: MacAddress,
    virtio: Virtio,
    tx_ring_len: usize,
    tx_ring_index: usize,
    tx_buffer: VAddr,
    _rx_buffer: VAddr,
}

impl VirtioNet {
    pub fn new(transport: Arc<dyn VirtioTransport>) -> Result<VirtioNet, VirtioAttachError> {
        let mut virtio = Virtio::new(transport);
        virtio.initialize(VIRTIO_NET_F_MAC, 2 /* RX and TX queues. */)?;

        // Read the MAC address.
        let mut mac_addr = [0; 6];
        for (i, byte) in mac_addr.iter_mut().enumerate() {
            *byte = virtio.read_device_config8((offset_of!(VirtioNetConfig, mac) + i) as u16);
        }
        info!(
            "virtio-net: MAC address is {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5],
        );

        let tx_ring_len = virtio.virtq(VIRTIO_NET_QUEUE_RX).num_descs() as usize;
        let rx_ring_len = virtio.virtq(VIRTIO_NET_QUEUE_TX).num_descs() as usize;
        let tx_buffer = alloc_pages(
            (align_up(PACKET_LEN_MAX * tx_ring_len, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL,
        )
        .unwrap()
        .as_vaddr();
        let rx_buffer = alloc_pages(
            (align_up(PACKET_LEN_MAX * rx_ring_len, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL,
        )
        .unwrap()
        .as_vaddr();

        let rx_virtq = virtio.virtq_mut(VIRTIO_NET_QUEUE_RX);
        for i in 0..rx_ring_len {
            rx_virtq.enqueue(&[super::virtio::VirtqDescBuffer::WritableFromDevice {
                addr: rx_buffer.add(i * PACKET_LEN_MAX).as_paddr(),
                len: PACKET_LEN_MAX,
            }])
        }

        Ok(VirtioNet {
            mac_addr: MacAddress::new(mac_addr),
            virtio,
            tx_buffer,
            _rx_buffer: rx_buffer,
            tx_ring_len,
            tx_ring_index: 0,
        })
    }

    pub fn handle_irq(&mut self) {
        if !self
            .virtio
            .read_isr_status()
            .contains(IsrStatus::QUEUE_INTR)
        {
            return;
        }

        let rx_virtq = self.virtio.virtq_mut(VIRTIO_NET_QUEUE_RX);

        while let Some(VirtqUsedChain { descs, total_len }) = rx_virtq.pop_used() {
            debug_assert!(descs.len() == 1);
            let addr = match descs[0] {
                super::virtio::VirtqDescBuffer::WritableFromDevice { addr, .. } => addr,
                super::virtio::VirtqDescBuffer::ReadOnlyFromDevice { .. } => unreachable!(),
            };

            if total_len < size_of::<VirtioNetHeader>() {
                warn!("virtio-net: received a too short buffer, ignoring...");
                continue;
            }

            trace!(
                "virtio-net: received {} octets (paddr={}, payload_len={})",
                total_len,
                addr,
                total_len - size_of::<VirtioNetHeader>()
            );

            let buffer = unsafe {
                core::slice::from_raw_parts(
                    addr.as_ptr::<u8>().add(size_of::<VirtioNetHeader>()),
                    total_len - size_of::<VirtioNetHeader>(),
                )
            };
            receive_ethernet_frame(buffer);

            rx_virtq.enqueue(&[super::virtio::VirtqDescBuffer::WritableFromDevice {
                addr,
                len: PACKET_LEN_MAX,
            }])
        }
    }

    fn mac_addr(&self) -> MacAddress {
        self.mac_addr
    }

    fn transmit(&mut self, frame: &[u8]) {
        let i = self.tx_ring_index % self.tx_ring_len;
        let addr = self.tx_buffer.add(i * PACKET_LEN_MAX);

        trace!(
            "virtio-net: transmitting {} octets (tx_ring={}, paddr={})",
            frame.len(),
            i,
            addr.as_paddr()
        );

        // Fill the virtio-net header.
        let header_len = size_of::<VirtioNetHeader>();
        assert!(frame.len() <= PACKET_LEN_MAX - header_len);
        let header = unsafe { &mut *addr.as_mut_ptr::<VirtioNetHeader>() };
        header.flags = 0;
        header.gso_type = 0;
        header.gso_size = 0;
        header.checksum_start = 0;
        header.checksum_offset = 0;
        header.num_buffer = 1;

        // Copy the payload into the our buffer.
        unsafe {
            addr.as_mut_ptr::<u8>()
                .add(header_len)
                .copy_from_nonoverlapping(frame.as_ptr(), frame.len());
        }

        // Construct a descriptor chain.
        let chain = &[super::virtio::VirtqDescBuffer::ReadOnlyFromDevice {
            addr: addr.as_paddr(),
            len: header_len + frame.len(),
        }];

        // Enqueue the transmission request and kick the device.
        let tx_virtq = self.virtio.virtq_mut(VIRTIO_NET_QUEUE_TX);
        tx_virtq.enqueue(chain);
        tx_virtq.notify();

        self.tx_ring_index += 1;
    }
}

struct VirtioNetDriver {
    device: Arc<SpinLock<VirtioNet>>,
}

impl VirtioNetDriver {
    fn new(device: Arc<SpinLock<VirtioNet>>) -> VirtioNetDriver {
        VirtioNetDriver { device }
    }
}

impl Driver for VirtioNetDriver {
    fn name(&self) -> &str {
        "virtio-net"
    }
}

impl EthernetDriver for VirtioNetDriver {
    fn mac_addr(&self) -> MacAddress {
        self.device.lock().mac_addr()
    }

    fn transmit(&self, frame: &[u8]) {
        self.device.lock().transmit(frame);
    }
}

pub struct VirtioNetBuilder {}
impl VirtioNetBuilder {
    pub fn new() -> VirtioNetBuilder {
        VirtioNetBuilder {}
    }
}

impl DriverBuilder for VirtioNetBuilder {
    fn attach_pci(&self, pci_device: &PciDevice) {
        // Check if the device is a network card ("4.1.2 PCI Device Discovery").
        if pci_device.config().vendor_id() == 0x1af4
            && pci_device.config().device_id() != 0x1040 + 1
        {
            return;
        }

        trace!("virtio-net: found the device (over PCI)");
        let device = match VirtioPci::attach_pci(pci_device, VirtioNet::new) {
            Ok(device) => Arc::new(SpinLock::new(device)),
            Err(VirtioAttachError::InvalidVendorId) => {
                // Not a virtio-net device.
                return;
            }
            Err(err) => {
                warn!("failed to attach a virtio-net: {:?}", err);
                return;
            }
        };

        register_ethernet_driver(Box::new(VirtioNetDriver::new(device.clone())));
        attach_irq(pci_device.config().interrupt_line(), move || {
            device.lock().handle_irq();
        });
    }

    fn attach_virtio_mmio(&self, mmio_device: &VirtioMmioDevice) {
        let mmio = mmio_device.mmio_base.as_vaddr();
        let magic = unsafe { *mmio.as_ptr::<u32>() };
        let virtio_version = unsafe { *mmio.add(4).as_ptr::<u32>() };
        let device_id = unsafe { *mmio.add(8).as_ptr::<u32>() };

        if magic != 0x74726976 {
            return;
        }

        if virtio_version != 2 {
            warn!("unsupported virtio device version: {}", virtio_version);
            return;
        }

        // It looks like a virtio device. Check if the device is a network card.
        if device_id != 1 {
            return;
        }

        trace!("virtio-net: found the device (over MMIO)");

        let transport = Arc::new(VirtioMmio::new(mmio_device.mmio_base));
        let device = match VirtioNet::new(transport) {
            Ok(device) => Arc::new(SpinLock::new(device)),
            Err(VirtioAttachError::InvalidVendorId) => {
                // Not a virtio-net device.
                return;
            }
            Err(err) => {
                warn!("failed to attach a virtio-net: {:?}", err);
                return;
            }
        };

        register_ethernet_driver(Box::new(VirtioNetDriver::new(device.clone())));
        attach_irq(mmio_device.irq, move || {
            device.lock().handle_irq();
        });
    }
}

pub fn init() {
    register_driver_builder(Box::new(VirtioNetBuilder::new()));
}
