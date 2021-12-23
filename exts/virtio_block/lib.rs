// A virtio-block driver
#![no_std]

extern crate alloc;
#[macro_use]
extern crate kerla_api;

use core::mem::size_of;

use alloc::boxed::Box;
use kerla_api::address::VAddr;
use kerla_api::arch::PAGE_SIZE;
use kerla_api::driver::block::{register_block_driver, BlockDriver};
use kerla_api::driver::{attach_irq, register_driver_prober, DeviceProber, Driver};
use kerla_api::mm::{alloc_pages, AllocPageFlags};
use kerla_api::{info, warn};

use kerla_api::sync::SpinLock;
use kerla_utils::alignment::align_up;
use kerla_utils::byte_size::ByteSize;

use memoffset::offset_of;

use alloc::sync::Arc;

use virtio::device::{IsrStatus, Virtio, VirtqDescBuffer, VirtqUsedChain};

use virtio::transports::{
    virtio_mmio::VirtioMmio, virtio_pci_legacy::VirtioLegacyPci,
    virtio_pci_modern::VirtioModernPci, VirtioAttachError, VirtioTransport,
};

const VIRTIO_BLK_F_SIZE: u64 = 1 << 6;
const VIRTIO_REQUEST_QUEUE: u16 = 0;
const MAX_BLK_SIZE: usize = 512;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct VirtioBlockConfig {
    capacity: u64,
    size_max: u32,
    seg_max: u32,
    cylinders: u16,
    heads: u8,
    sectors: u8,
    block_size: u32,
    physical_block_exp: u8,
    alignment_offset: u8,
    min_io_size: u16,
    opt_io_size: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct VirtioBlockRequest {
    type_: u32,
    reserved: u32,
    sector: u64,
}

#[repr(u32)]
enum RequestType {
    Read = 0,
    Write = 1,
}

pub struct VirtioBlock {
    virtio: Virtio,
    request_buffer: VAddr,
    read_buffer: VAddr,
    status_buffer: VAddr,
    request_buffer_index: usize,
    ring_len: usize
}

impl VirtioBlock {
    pub fn new(transport: Arc<dyn VirtioTransport>) -> Result<VirtioBlock, VirtioAttachError> {
        let mut virtio = Virtio::new(transport);
        virtio.initialize(VIRTIO_BLK_F_SIZE, 1)?;
        // Read the block size
        let block_size =
            virtio.read_device_config64(offset_of!(VirtioBlockConfig, capacity) as u16);

        info!(
            "The disk capacity is {}",
            ByteSize::new(block_size as usize * 512)
        );

        let ring_len = virtio.virtq(VIRTIO_REQUEST_QUEUE).num_descs() as usize;

        let request_buffer = alloc_pages(
            (align_up(MAX_BLK_SIZE * ring_len, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL,
        )
        .unwrap()
        .as_vaddr();

        let status_buffer = alloc_pages(
            (align_up((MAX_BLK_SIZE * ring_len) + 1, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL,
        )
        .unwrap()
        .as_vaddr();
        
        let read_buffer = alloc_pages(
            (align_up((MAX_BLK_SIZE * ring_len) + 2, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL,
        )
        .unwrap()
        .as_vaddr();

        Ok(VirtioBlock {
            virtio: virtio,
            request_buffer: request_buffer,
            status_buffer: status_buffer,
            read_buffer: read_buffer,
            request_buffer_index: 0,
            ring_len: ring_len
        })
    }

    fn write_to_device(&mut self, sector: u64, buf: &[u8]) {
        let i = self.request_buffer_index % self.ring_len;
        let request_addr = self.request_buffer.add(MAX_BLK_SIZE * i);
        let status_addr = self.status_buffer.add((MAX_BLK_SIZE * i) + 1);
        let read_addr = self.read_buffer.add((MAX_BLK_SIZE * i) + 2);
        let request_header_len = size_of::<VirtioBlockRequest>();
       

        // Fill block request
        let block_request = unsafe { &mut *request_addr.as_mut_ptr::<VirtioBlockRequest>() };
        block_request.type_ = 1;
        block_request.sector = sector;
        block_request.reserved = 0;

        // Chain Descriptor
        let chain = &[
            VirtqDescBuffer::ReadOnlyFromDevice {
                addr: request_addr.as_paddr(),
                len: request_header_len,
            },
            VirtqDescBuffer::ReadOnlyFromDevice {
                addr: read_addr.as_paddr(),
                len: buf.len(),
            },
            VirtqDescBuffer::WritableFromDevice {
                addr: status_addr.as_paddr(),
                len: 1,
            },
        ];

        // enqueue to data to send
        let request_virtq = self.virtio.virtq_mut(VIRTIO_REQUEST_QUEUE);
        request_virtq.enqueue(chain);
        request_virtq.notify();
    }

    pub fn handle_irq(&mut self) {
        if !self
            .virtio
            .read_isr_status()
            .contains(IsrStatus::QUEUE_INTR)
        {
            return;
        }

        let request_virtq = self.virtio.virtq_mut(VIRTIO_REQUEST_QUEUE);

        while let Some(VirtqUsedChain {
            descs,
            total_len: _,
        }) = request_virtq.pop_used()
        {
            debug_assert!(descs.len() == 1);
            let addr = match descs[0] {
                VirtqDescBuffer::WritableFromDevice { addr, .. } => addr,
                VirtqDescBuffer::ReadOnlyFromDevice { .. } => unreachable!(),
            };

            request_virtq.enqueue(&[VirtqDescBuffer::WritableFromDevice {
                addr,
                len: MAX_BLK_SIZE,
            }])
        }
    }
}

struct VirtioBlockDriver {
    device: Arc<SpinLock<VirtioBlock>>,
}

impl VirtioBlockDriver {
    fn new(device: Arc<SpinLock<VirtioBlock>>) -> VirtioBlockDriver {
        VirtioBlockDriver { device: device }
    }
}

impl Driver for VirtioBlockDriver {
    fn name(&self) -> &str {
        "virtio-blk-pci"
    }
}

impl BlockDriver for VirtioBlockDriver {
    fn read_block(&self, sector: u64, frame: &[u8]) {
       todo!()
    }

    fn write_block(&self, sector: u64, frame: &[u8]) {
       todo!()
    }
}

pub struct VirtioBlockProber;

impl VirtioBlockProber {
    pub fn new() -> VirtioBlockProber {
        VirtioBlockProber {}
    }
}

impl DeviceProber for VirtioBlockProber {
    fn probe_pci(&self, pci_device: &kerla_api::driver::pci::PciDevice) {
        // Check if the device is a block device ("4.1.2 PCI Device Discovery").
        if pci_device.config().vendor_id() != 0x1af4 {
            return;
        }

        // Check if the it's a legacy or traditional device.
        let device_id = pci_device.config().device_id();
        if device_id != 0x1040 + 2 && device_id != 0x1001 {
            return;
        }

        trace!("virtio-blk-pci: found the device (over PCI)");
        let transport = match VirtioModernPci::probe_pci(pci_device) {
            Ok(transport) => transport,
            Err(VirtioAttachError::InvalidVendorId) => {
                // Not a virtio-net device.
                return;
            }
            Err(err) => {
                trace!("failed to attach a virtio-blk-pci as a modern device: {:?}, falling back to the legacy driver", err);
                match VirtioLegacyPci::probe_pci(pci_device) {
                    Ok(transport) => transport,
                    Err(err) => {
                        warn!(
                            "failed to attach a virtio-net as a legacy device: {:?}",
                            err
                        );
                        return;
                    }
                }
            }
        };

        let virtio = match VirtioBlock::new(transport) {
            Ok(virtio) => virtio,
            Err(err) => {
                warn!("failed to initialize virtio-blk-pci: {:?}", err);
                return;
            }
        };

        let device = Arc::new(SpinLock::new(virtio));
        register_block_driver(Box::new(VirtioBlockDriver::new(device.clone())));
        attach_irq(pci_device.config().interrupt_line(), move || {
            device.lock().handle_irq();
        });
    }

    fn probe_virtio_mmio(&self, mmio_device: &kerla_api::driver::VirtioMmioDevice) {
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

        // Device is a block device
        if device_id != 2 {
            return;
        }

        trace!("virtio-block: found the device (over MMIO)");

        let transport = Arc::new(VirtioMmio::new(mmio_device.mmio_base));
        let device = match VirtioBlock::new(transport) {
            Ok(device) => Arc::new(SpinLock::new(device)),
            Err(VirtioAttachError::InvalidVendorId) => {
                // Not a virtio-block device.
                return;
            }
            Err(err) => {
                warn!("failed to attach a virtio-block: {:?}", err);
                return;
            }
        };

        register_block_driver(Box::new(VirtioBlockDriver::new(device.clone())));
        attach_irq(mmio_device.irq, move || {
            device.lock().handle_irq();
        });
    }
}

pub fn init() {
    register_driver_prober(Box::new(VirtioBlockProber::new()));
}
