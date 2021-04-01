//! A virtio-net device driver.
use super::{
    attach_irq,
    pci::PciDevice,
    register_ethernet_driver,
    virtio::{IsrStatus, Virtio},
    DriverBuilder, MacAddress,
};
use super::{Driver, EthernetDriver};
use crate::{
    arch::{SpinLock, VAddr, PAGE_SIZE},
    mm::page_allocator::{alloc_pages, AllocPageFlags},
    result::{Errno, Error, Result},
};
use crate::{drivers::ioport::IoPort, net::receive_ethernet_frame};
use alloc::sync::Arc;
use core::mem::size_of;
use penguin_utils::alignment::align_up;
const VIRTIO_NET_F_MAC: u32 = 1 << 5;

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
    pub fn new(ioport: IoPort) -> Result<VirtioNet> {
        let mut virtio = Virtio::new(ioport);
        virtio.initialize(VIRTIO_NET_F_MAC, 2 /* RX and TX queues. */)?;

        // Read the MAC address.
        let mut mac_addr = [0; 6];
        for (i, byte) in mac_addr.iter_mut().enumerate() {
            *byte = virtio
                .read_device_config8((memoffset::offset_of!(VirtioNetConfig, mac) + i) as u16);
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
            rx_virtq
                .enqueue(&[super::virtio::VirtqDescBuffer::WritableFromDevice {
                    addr: rx_buffer.add(i * PACKET_LEN_MAX).as_paddr(),
                    len: PACKET_LEN_MAX,
                }])
                .unwrap();
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
        info!("virtio IRQ: entering");

        while let Some(used) = rx_virtq.pop_used() {
            info!("virtio IRQ: pop a desc, len={}", used.total_len);
            debug_assert!(used.descs.len() == 1);
            let (addr, len) = match used.descs[0] {
                super::virtio::VirtqDescBuffer::WritableFromDevice { addr, len } => (addr, len),
                super::virtio::VirtqDescBuffer::ReadOnlyFromDevice { .. } => unreachable!(),
            };

            let buffer = unsafe {
                core::slice::from_raw_parts(
                    addr.as_ptr::<u8>().add(size_of::<VirtioNetHeader>()),
                    len - size_of::<VirtioNetHeader>(),
                )
            };
            receive_ethernet_frame(buffer);

            warn_if_err!(
                rx_virtq.enqueue(&[super::virtio::VirtqDescBuffer::WritableFromDevice {
                    addr,
                    len: PACKET_LEN_MAX,
                }])
            );
        }
    }
}

impl Driver for VirtioNet {
    fn name(&self) -> &str {
        "virtio-net"
    }
}

impl EthernetDriver for VirtioNet {
    fn mac_addr(&self) -> Result<MacAddress> {
        Ok(self.mac_addr)
    }

    fn transmit(&mut self, frame: &[u8]) -> Result<()> {
        let i = self.tx_ring_index % self.tx_ring_len;
        let addr = self.tx_buffer.add(i * PACKET_LEN_MAX);

        info!(
            "virtio-net: transmit {} octets (tx_ring={}, paddr={})",
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
        tx_virtq.enqueue(chain)?;
        tx_virtq.notify();

        self.tx_ring_index += 1;
        Ok(())
    }
}

pub struct VirtioNetBuilder {}
impl VirtioNetBuilder {
    pub fn new() -> VirtioNetBuilder {
        VirtioNetBuilder {}
    }
}

impl DriverBuilder for VirtioNetBuilder {
    fn attach_pci(&self, pci_device: &PciDevice) -> Result<()> {
        if pci_device.config().vendor_id() != 0x1af4 && pci_device.config().device_id() != 0x1000 {
            return Err(Error::new(Errno::EINVAL));
        }

        let ioport = match pci_device.config().bar0() {
            super::pci::Bar::IOMapped { port } => IoPort::new(port),
            bar0 => {
                warn!("virtio: unsupported type of BAR#0: {:x?}", bar0);
                return Err(Error::new(Errno::EINVAL));
            }
        };

        trace!("virtio-net: found the device");
        pci_device.enable_bus_master();

        let driver = Arc::new(SpinLock::new(VirtioNet::new(ioport)?));
        register_ethernet_driver(driver.clone());

        attach_irq(pci_device.config().interrupt_line(), move || {
            driver.lock().handle_irq();
        });

        Ok(())
    }
}
