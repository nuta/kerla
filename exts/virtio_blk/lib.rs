#![no_std]

extern crate alloc;
#[macro_use]
extern crate kerla_api;

use core::mem::size_of;

use alloc::boxed::Box;
use kerla_api::address::VAddr;
use kerla_api::driver::{DeviceProber, Driver, register_driver_prober};
use kerla_api::driver::block::{BlockDriver, register_block_driver};
use kerla_api::{info, warn};
use kerla_api::mm::{alloc_pages, AllocPageFlags};
use kerla_api::arch::PAGE_SIZE;

use kerla_api::sync::SpinLock;
use kerla_utils::alignment::align_up;

use memoffset::offset_of;

use alloc::sync::Arc;

use virtio::device::{IsrStatus, VirtQueue, Virtio, VirtqDescBuffer, VirtqUsedChain};

use virtio::transports::VirtioTransport;
use virtio::transports::virtio_mmio::VirtioMmio;
use virtio::transports::virtio_pci::{VirtioAttachError, VirtioPci};


const VIRTIO_BLK_F_SIZE: u64 = 1 << 6;
const VIRTIO_REQUEST_QUEUE: u16 = 0;
const MAX_BLK_SIZE: usize = 512;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct VirtioBlkConfig {
    capacity: u64,
    size_max: u32,
    seg_max: u32,
    cylinders: u16,
    heads: u8,
    sectors: u8,
    blk_size: u32,
    physical_block_exp: u8,
    alignment_offset: u8,
    min_io_size: u16,
    opt_io_size: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct VirtioBlkRequest {
    type_: u32,
    reserved: u32,
    sector: u64,
}



#[repr(u32)]
enum RequestType {
    Read = 0,
    Write = 1
}


pub struct VirtioBlk {
    virtio: Virtio,
    buffer: VAddr,
}

impl VirtioBlk {
    pub fn new(transport: Arc<dyn VirtioTransport>) -> Result<VirtioBlk, VirtioAttachError> {
        let virtio = Virtio::new(transport);
        // Read the block size
        let block_size = offset_of!(VirtioBlkConfig, capacity);
        // TODO: Make sure it returns the block size once qemu block device is enabled
        info!("Block size is {}", block_size);

        // Create buffer for virtqueue
        
        let ring_len = virtio.virtq(VIRTIO_REQUEST_QUEUE).num_descs() as usize;
        
        let buffer = alloc_pages(
            (align_up(MAX_BLK_SIZE * ring_len, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL 
        ).unwrap().as_vaddr();

        

        Ok(VirtioBlk {
            virtio: virtio,
            buffer: buffer
        
        })


    }

    fn request_to_device(&mut self, request_type: RequestType, sector: u64, frame: &[u8]) {
        let addr = self.buffer.add(MAX_BLK_SIZE);
        let request_len = size_of::<VirtioBlkRequest>();
        
        // Fill block request 
        let block_request = unsafe { &mut *addr.as_mut_ptr::<VirtioBlkRequest>()};
        block_request.type_ = request_type as u32;
        block_request.sector = sector;
        block_request.reserved = 0;
        
        // Copy data into buffer 
        unsafe {
            addr.as_mut_ptr::<u8>()
            .add(request_len)
            .copy_from_nonoverlapping(frame.as_ptr(), frame.len());

        }

        // Chain Descriptor 
        let chain = &[VirtqDescBuffer::ReadOnlyFromDevice {
            addr: addr.as_paddr(),
            len: request_len + frame.len()
        }];

        // enqueue to data to send 
        let request_virtq = self.virtio.virtq_mut(VIRTIO_REQUEST_QUEUE);
        request_virtq.enqueue(chain);
        request_virtq.notify();
    
    }

    pub fn handle_irq(&mut self) {

    }

}

struct VirtioBlkDriver {
    device: Arc<SpinLock<VirtioBlk>>
}

impl VirtioBlkDriver {
    fn new(device: Arc<SpinLock<VirtioBlk>>) -> VirtioBlkDriver {
        VirtioBlkDriver { device: device}
    }
}

impl Driver for VirtioBlkDriver {
    fn name(&self) -> &str {
        "virtio-blk"
    }
}

impl BlockDriver for VirtioBlkDriver {
    fn read_block(&self, sector: u64, frame: &[u8]) {
        self.device.lock().request_to_device(RequestType::Read, sector, frame)
    }

    fn write_block(&self, sector: u64, frame: &[u8]) {
        self.device.lock().request_to_device(RequestType::Write, sector, frame)
    }
}

pub struct VirtioBlkProber;

impl VirtioBlkProber {
    pub fn new() -> VirtioBlkProber {
        VirtioBlkProber {}
    }
}

impl DeviceProber for VirtioBlkProber {
    fn probe_pci(&self, pci_device: &kerla_api::driver::pci::PciDevice) {
        // Check if device is a block device 
        if pci_device.config().vendor_id() == 0x1af4 
        && pci_device.config().device_id() != 0x1042 {
            return;
        }
        trace!("virtio-blk: found the device (over PCI");
        let device = match VirtioPci::probe_pci(pci_device, VirtioBlk::new) {
            Ok(device) => Arc::new(SpinLock::new(device)),
            Err(VirtioAttachError::InvalidVendorId) => {
                // not a virtio-blk device 
                return;
            }
            Err(err) => {
                warn!("Failed to attach a virtio-blk: {:?}", err);
                return;
            }
        };

        register_block_driver(Box::new(VirtioBlkDriver::new(device.clone())))
        
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

        trace!("virtio-blk: found the device (over MMIO)");

        let transport = Arc::new(VirtioMmio::new(mmio_device.mmio_base));
        let device = match VirtioBlk::new(transport) {
            Ok(device) => Arc::new(SpinLock::new(device)),
            Err(VirtioAttachError::InvalidVendorId) => {
                // Not a virtio-blk device.
                return;
            }
            Err(err) => {
                warn!("failed to attach a virtio-blk: {:?}", err);
                return;
            }
        };

        register_block_driver(Box::new(VirtioBlkDriver::new(device.clone())))

    }
}

pub fn init() {
    register_driver_prober(Box::new(VirtioBlkProber::new()));
}
