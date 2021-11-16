extern crate alloc;

use kerla_api::address::VAddr;
use kerla_api::info;
use kerla_api::mm::{alloc_pages, AllocPageFlags};
use kerla_api::arch::PAGE_SIZE;

use kerla_utils::alignment::align_up;

use memoffset::offset_of;

use alloc::sync::Arc;

use virtio::device::Virtio;

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

#[repr(u32)]
enum RequestType {
    Read = 0,
    Write = 1
}


pub struct VirtioBlk {
    virtio: Virtio,
    read_buffer: VAddr,
    write_buffer: VAddr,
}

impl VirtioBlk {
    pub fn new(transport: Arc<dyn VirtioTransport>) -> Result<VirtioBlk, VirtioAttachError> {
        let mut virtio = Virtio::new(transport);
        virtio.initialize(VIRTIO_BLK_F_SIZE, 1);

        // Read the block size
        let block_size = offset_of!(VirtioBlkConfig, capacity);
        // TODO: Make sure it returns the block size 
        info!("Block size is {}", block_size);

        // Create buffers for virtqueue
        let ring_len = virtio.virtq(VIRTIO_REQUEST_QUEUE).num_descs() as usize;
        
        let read_buffer = alloc_pages(
            (align_up(MAX_BLK_SIZE * ring_len, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL 
        ).unwrap().as_vaddr();

        let write_buffer = alloc_pages(
            (align_up(MAX_BLK_SIZE * ring_len, PAGE_SIZE)) / PAGE_SIZE,
            AllocPageFlags::KERNEL 
        ).unwrap().as_vaddr();

        Ok(VirtioBlk {
            virtio: virtio,
            read_buffer: read_buffer,
            write_buffer: write_buffer
        }
        )


    }

    pub fn request_to_device(&mut self, request_type: RequestType) {

    }
}