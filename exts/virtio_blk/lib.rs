#![no_std]
extern crate alloc;

use core::mem::size_of;

use kerla_api::address::VAddr;
use kerla_api::driver::{DeviceProber, Driver};
use kerla_api::driver::block::BlockDriver;
use kerla_api::info;
use kerla_api::mm::{alloc_pages, AllocPageFlags};
use kerla_api::arch::PAGE_SIZE;

use kerla_api::sync::SpinLock;
use kerla_utils::alignment::align_up;

use memoffset::offset_of;

use alloc::sync::Arc;

use virtio::device::{IsrStatus, Virtio, VirtqDescBuffer};

use virtio::transports::VirtioTransport;
use virtio::transports::virtio_pci::VirtioAttachError;


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
        let mut virtio = Virtio::new(transport);

        // Read the block size
        let block_size = offset_of!(VirtioBlkConfig, capacity);
        // TODO: Make sure it returns the block size 
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
        
        }
        )


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

impl DeviceProber for VirtioBlkProber {
    fn probe_pci(&self, pci_device: &kerla_api::driver::pci::PciDevice) {
        
    }

    fn probe_virtio_mmio(&self, mmio_device: &kerla_api::driver::VirtioMmioDevice) {
        
    }
}

pub fn init() {

}