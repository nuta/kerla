//! A virtio device driver library.
use crate::drivers::ioport::IoPort;
use crate::result::{Errno, Result};
use crate::{
    arch::{PAddr, VAddr, PAGE_SIZE},
    mm::page_allocator::{alloc_pages, AllocPageFlags},
};
use alloc::vec::Vec;
use bitflags::bitflags;
use core::convert::TryInto;
use core::mem::size_of;
use core::sync::atomic::{self, Ordering};
use penguin_utils::alignment::align_up;

const VIRTIO_STATUS_ACK: u8 = 1;
const VIRTIO_STATUS_DRIVER: u8 = 2;
const VIRTIO_STATUS_DRIVER_OK: u8 = 4;
const VIRTIO_STATUS_FEAT_OK: u8 = 82;

const VIRTIO_REG_DEVICE_FEATS: u16 = 0x00;
const VIRTIO_REG_DRIVER_FEATS: u16 = 0x04;
const VIRTIO_REG_QUEUE_ADDR_PFN: u16 = 0x08;
const VIRTIO_REG_NUM_DESCS: u16 = 0x0c;
const VIRTIO_REG_QUEUE_SELECT: u16 = 0x0e;
const VIRTIO_REG_QUEUE_NOTIFY: u16 = 0x10;
const VIRTIO_REG_DEVICE_STATUS: u16 = 0x12;
const VIRTIO_REG_ISR_STATUS: u16 = 0x13;
const VIRTIO_REG_DEVICE_CONFIG_BASE: u16 = 0x14;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

impl VirtqDesc {
    pub fn is_writable(&self) -> bool {
        self.flags & VIRTQ_DESC_F_WRITE != 0
    }

    pub fn has_next(&self) -> bool {
        self.flags & VIRTQ_DESC_F_NEXT != 0
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtqAvail {
    flags: u16,
    index: u16,
    // The rings (an array of descriptor indices) immediately follows here.
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct VirtqUsed {
    flags: u16,
    index: u16,
    // The rings (an array of VirtqUsedElem) immediately follows here.
}

pub enum VirtqDescBuffer {
    ReadOnlyFromDevice { addr: PAddr, len: usize },
    WritableFromDevice { addr: PAddr, len: usize },
}

pub struct VirtqUsedChain {
    pub descs: Vec<VirtqDescBuffer>,
    pub total_len: usize,
}

/// A virtqueue.
pub struct VirtQueue {
    index: u16,
    ioport: IoPort,
    num_descs: u16,
    last_used_index: u16,
    free_head: u16,
    num_free_descs: u16,
    descs: VAddr,
    avail: VAddr,
    used: VAddr,
}

impl VirtQueue {
    pub fn new(index: u16, ioport: IoPort) -> VirtQueue {
        ioport.write16(VIRTIO_REG_QUEUE_SELECT, index);
        let num_descs = ioport.read16(VIRTIO_REG_NUM_DESCS);
        let avail_ring_off = size_of::<VirtqDesc>() * (num_descs as usize);
        let avail_ring_size: usize = size_of::<u16>() * (3 + (num_descs as usize));
        let used_ring_off = align_up(avail_ring_off + avail_ring_size, PAGE_SIZE);
        let used_ring_size =
            size_of::<u16>() * 3 + size_of::<VirtqUsedElem>() * (num_descs as usize);
        let virtq_size = used_ring_off + align_up(used_ring_size, PAGE_SIZE);

        let virtqueue_paddr = alloc_pages(
            align_up(virtq_size, PAGE_SIZE) / PAGE_SIZE,
            AllocPageFlags::KERNEL | AllocPageFlags::ZEROED,
        )
        .expect("failed to allocate virtuqeue");

        ioport.write32(
            VIRTIO_REG_QUEUE_ADDR_PFN,
            (virtqueue_paddr.value() / PAGE_SIZE) as u32,
        );

        // Add descriptors into the free list.
        let descs = virtqueue_paddr.as_vaddr();
        for i in 0..num_descs {
            let desc = unsafe { &mut *descs.as_mut_ptr::<VirtqDesc>().offset(i as isize) };
            desc.next = if i == num_descs - 1 { 0 } else { i + 1 };
        }

        VirtQueue {
            index,
            ioport,
            num_descs,
            last_used_index: 0,
            free_head: 0,
            num_free_descs: num_descs,
            descs,
            avail: virtqueue_paddr.add(avail_ring_off).as_vaddr(),
            used: virtqueue_paddr.add(used_ring_off).as_vaddr(),
        }
    }

    /// Enqueues a request to the device. A request is a chain of descriptors
    /// (e.g. `struct virtio_blk_req` as defined in the spec).
    ///
    /// Once you've enqueued all requests, you need to notify the device through
    /// the `notify` method.
    pub fn enqueue(&mut self, chain: &[VirtqDescBuffer]) -> Result<()> {
        debug_assert!(!chain.is_empty());

        // Check if we have the enough number of free descriptors.
        if (self.num_free_descs as usize) < chain.len() {
            return Err(Errno::ENOMEM.into());
        }

        let head_index = self.free_head;
        let mut desc_index = self.free_head;
        for (i, buffer) in chain.iter().enumerate() {
            let desc = self.desc_mut(desc_index);
            let (addr, len, flags) = match buffer {
                VirtqDescBuffer::ReadOnlyFromDevice { addr, len } => (addr, *len, 0),
                VirtqDescBuffer::WritableFromDevice { addr, len } => {
                    (addr, *len, VIRTQ_DESC_F_WRITE)
                }
            };

            desc.addr = addr.value() as u64;
            desc.len = len.try_into().unwrap();
            desc.flags = flags;

            if i == chain.len() - 1 {
                let unused_next = desc.next;
                desc.next = 0;
                self.free_head = unused_next;
                self.num_free_descs -= chain.len() as u16;
            } else {
                desc.flags |= VIRTQ_DESC_F_NEXT;
                desc_index = desc.next;
            }
        }

        let avail_elem_index = self.avail().index % self.num_descs;
        *self.avail_elem_mut(avail_elem_index) = head_index;
        self.avail_mut().index += 1;

        Ok(())
    }

    /// Notifies the device to start processing descriptors.
    pub fn notify(&self) {
        atomic::fence(Ordering::Release);
        self.ioport.write16(VIRTIO_REG_QUEUE_NOTIFY, self.index);
    }

    /// Returns a chain of descriptors processed by the device.
    pub fn pop_used(&mut self) -> Option<VirtqUsedChain> {
        if self.last_used_index == self.used().index {
            return None;
        }

        let head = *self.used_elem(self.last_used_index);
        self.last_used_index += 1;

        let mut used_descs = Vec::new();
        let mut next_desc_index = head.id as u16;
        let mut num_descs_in_chain = 1;
        let current_free_head = self.free_head;
        loop {
            let desc = self.desc_mut(next_desc_index);
            used_descs.push(if desc.is_writable() {
                VirtqDescBuffer::WritableFromDevice {
                    addr: PAddr::new(desc.addr as usize),
                    len: desc.len as usize,
                }
            } else {
                VirtqDescBuffer::ReadOnlyFromDevice {
                    addr: PAddr::new(desc.addr as usize),
                    len: desc.len as usize,
                }
            });

            if !desc.has_next() {
                // Prepend the popped chain into the free list.
                desc.next = current_free_head;
                self.free_head = head.id as u16;
                self.num_free_descs += num_descs_in_chain;
                break;
            }

            next_desc_index = desc.next;
            num_descs_in_chain += 1;
        }

        Some(VirtqUsedChain {
            total_len: head.len as usize,
            descs: used_descs,
        })
    }

    /// Returns the defined number of descriptors in the virtqueue.
    pub fn num_descs(&self) -> u16 {
        self.num_descs
    }

    fn desc_mut(&mut self, index: u16) -> &mut VirtqDesc {
        debug_assert!(index < self.num_descs);
        unsafe { &mut *self.descs.as_mut_ptr::<VirtqDesc>().offset(index as isize) }
    }

    fn avail(&self) -> &VirtqAvail {
        unsafe { &*self.avail.as_ptr::<VirtqAvail>() }
    }

    fn avail_mut(&mut self) -> &mut VirtqAvail {
        unsafe { &mut *self.avail.as_mut_ptr::<VirtqAvail>() }
    }

    fn avail_elem_mut(&mut self, index: u16) -> &mut u16 {
        debug_assert!(index < self.num_descs);
        unsafe {
            &mut *self
                .avail
                .add(size_of::<VirtqAvail>())
                .as_mut_ptr::<u16>()
                .offset(index as isize)
        }
    }

    fn used(&self) -> &VirtqUsed {
        unsafe { &*self.used.as_ptr::<VirtqUsed>() }
    }

    fn used_elem(&self, index: u16) -> &VirtqUsedElem {
        debug_assert!(index < self.num_descs);
        unsafe {
            &*self
                .used
                .add(size_of::<VirtqUsed>())
                .as_ptr::<VirtqUsedElem>()
                .offset(index as isize)
        }
    }
}

bitflags! {
    pub struct IsrStatus: u8 {
        const QUEUE_INTR = 1 << 0;
        const DEVICE_CONFIG_INTR = 1 << 1;
    }
}

pub struct Virtio {
    ioport: IoPort,
    virtqueues: Vec<VirtQueue>,
}

impl Virtio {
    pub fn new(ioport: IoPort) -> Virtio {
        Virtio {
            ioport,
            virtqueues: Vec::new(),
        }
    }

    /// Initialize the virtio device. It aborts if any of the features is not
    /// supported.
    pub fn initialize(&mut self, features: u32, num_virtqueues: u16) -> Result<()> {
        // "3.1.1 Driver Requirements: Device Initialization"
        self.write_device_status(0); // Reset the device.
        self.write_device_status(self.read_device_status() | VIRTIO_STATUS_ACK);
        self.write_device_status(self.read_device_status() | VIRTIO_STATUS_DRIVER);

        if (self.read_device_features() & features) != features {
            return Err(Errno::EINVAL.into());
        }

        self.write_driver_features(features);
        self.write_device_status(self.read_device_status() | VIRTIO_STATUS_FEAT_OK);

        if (self.read_device_status() & VIRTIO_STATUS_FEAT_OK) == 0 {
            return Err(Errno::EINVAL.into());
        }

        // Initialize virtqueues.
        let mut virtqueues = Vec::new();
        for index in 0..num_virtqueues {
            virtqueues.push(VirtQueue::new(index, self.ioport));
        }
        self.virtqueues = virtqueues;

        self.write_device_status(self.read_device_status() | VIRTIO_STATUS_DRIVER_OK);
        Ok(())
    }

    /// Returns the `i`-th virtqueue.
    pub fn virtq(&self, i: u16) -> &VirtQueue {
        self.virtqueues.get(i as usize).unwrap()
    }

    /// Returns the `i`-th virtqueue.
    pub fn virtq_mut(&mut self, i: u16) -> &mut VirtQueue {
        self.virtqueues.get_mut(i as usize).unwrap()
    }

    pub fn read_device_config8(&self, offset: u16) -> u8 {
        self.ioport.read8(VIRTIO_REG_DEVICE_CONFIG_BASE + offset)
    }

    pub fn read_isr_status(&self) -> IsrStatus {
        IsrStatus::from_bits(self.ioport.read8(VIRTIO_REG_ISR_STATUS)).unwrap()
    }

    fn read_device_status(&self) -> u8 {
        self.ioport.read8(VIRTIO_REG_DEVICE_STATUS)
    }

    fn write_device_status(&self, value: u8) {
        self.ioport.write8(VIRTIO_REG_DEVICE_STATUS, value);
    }

    fn read_device_features(&self) -> u32 {
        self.ioport.read32(VIRTIO_REG_DEVICE_FEATS)
    }

    fn write_driver_features(&self, value: u32) {
        self.ioport.write32(VIRTIO_REG_DRIVER_FEATS, value)
    }
}
