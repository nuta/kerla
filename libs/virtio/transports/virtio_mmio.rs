use crate::device::IsrStatus;

use kerla_api::address::{PAddr, VAddr};

use super::VirtioTransport;

pub struct VirtioMmio {
    mmio_base: VAddr,
}

impl VirtioMmio {
    pub fn new(mmio_base: PAddr) -> VirtioMmio {
        VirtioMmio {
            mmio_base: mmio_base.as_vaddr(),
        }
    }
}

impl VirtioTransport for VirtioMmio {
    fn is_modern(&self) -> bool {
        true
    }

    fn read_device_config8(&self, offset: u16) -> u8 {
        unsafe {
            self.mmio_base
                .add((0x100 + offset) as usize)
                .read_volatile::<u8>()
        }
    }

    fn read_isr_status(&self) -> IsrStatus {
        IsrStatus::from_bits(unsafe { self.mmio_base.add(0x60).read_volatile::<u32>() as u8 })
            .unwrap()
    }

    fn read_device_status(&self) -> u8 {
        unsafe { self.mmio_base.add(0x70).read_volatile::<u32>() as u8 }
    }

    fn write_device_status(&self, value: u8) {
        unsafe {
            self.mmio_base.add(0x70).write_volatile::<u32>(value as u32);
        }
    }

    fn read_device_features(&self) -> u64 {
        unsafe {
            self.mmio_base.add(0x14).write_volatile::<u32>(0);
            let low = self.mmio_base.add(0x10).read_volatile::<u32>();
            self.mmio_base.add(0x14).write_volatile::<u32>(1);
            let high = self.mmio_base.add(0x10).read_volatile::<u32>();
            ((high as u64) << 32) | (low as u64)
        }
    }

    fn write_driver_features(&self, value: u64) {
        unsafe {
            self.mmio_base.add(0x24).write_volatile::<u32>(0);
            self.mmio_base
                .add(0x20)
                .write_volatile::<u32>((value & 0xffff_ffff) as u32);
            self.mmio_base.add(0x24).write_volatile::<u32>(1);
            self.mmio_base
                .add(0x20)
                .write_volatile::<u32>((value >> 32) as u32);
        }
    }

    fn select_queue(&self, index: u16) {
        unsafe {
            self.mmio_base.add(0x30).write_volatile::<u32>(index as u32);
        }
    }

    fn queue_max_size(&self) -> u16 {
        unsafe { self.mmio_base.add(0x34).read_volatile::<u32>() as u16 }
    }

    fn set_queue_size(&self, queue_size: u16) {
        unsafe {
            self.mmio_base
                .add(0x38)
                .write_volatile::<u32>(queue_size as u32)
        }
    }

    fn notify_queue(&self, index: u16) {
        unsafe {
            self.mmio_base.add(0x50).write_volatile::<u32>(index as u32);
        }
    }

    fn enable_queue(&self) {
        unsafe {
            self.mmio_base.add(0x44).write_volatile::<u32>(1);
        }
    }

    fn set_queue_desc_paddr(&self, paddr: PAddr) {
        unsafe {
            self.mmio_base
                .add(0x80)
                .write_volatile::<u32>((paddr.value() & 0xffff_ffff) as u32);
            self.mmio_base
                .add(0x84)
                .write_volatile::<u32>((paddr.value() >> 32) as u32);
        }
    }

    fn set_queue_device_paddr(&self, paddr: PAddr) {
        unsafe {
            self.mmio_base
                .add(0xa0)
                .write_volatile::<u32>((paddr.value() & 0xffff_ffff) as u32);
            self.mmio_base
                .add(0xa4)
                .write_volatile::<u32>((paddr.value() >> 32) as u32);
        }
    }

    fn set_queue_driver_paddr(&self, paddr: PAddr) {
        unsafe {
            self.mmio_base
                .add(0x90)
                .write_volatile::<u32>((paddr.value() & 0xffff_ffff) as u32);
            self.mmio_base
                .add(0x94)
                .write_volatile::<u32>((paddr.value() >> 32) as u32);
        }
    }
}
