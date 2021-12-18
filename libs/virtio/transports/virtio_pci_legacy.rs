use crate::device::IsrStatus;

use alloc::sync::Arc;
use kerla_api::{
    address::PAddr,
    arch::PAGE_SIZE,
    driver::{
        ioport::IoPort,
        pci::{Bar, PciDevice},
    },
};

use super::{VirtioAttachError, VirtioTransport};

const REG_DEVICE_FEATS: u16 = 0x00;
const REG_DRIVER_FEATS: u16 = 0x04;
const REG_QUEUE_ADDR_PFN: u16 = 0x08;
const REG_QUEUE_SIZE: u16 = 0x0c;
const REG_QUEUE_SELECT: u16 = 0x0e;
const REG_QUEUE_NOTIFY: u16 = 0x10;
const REG_DEVICE_STATUS: u16 = 0x12;
const REG_ISR_STATUS: u16 = 0x13;
const REG_DEVICE_CONFIG_BASE: u16 = 0x14;

pub struct VirtioLegacyPci {
    port_base: IoPort,
}

impl VirtioLegacyPci {
    pub fn probe_pci(
        pci_device: &PciDevice,
    ) -> Result<Arc<dyn VirtioTransport>, VirtioAttachError> {
        // TODO: Check device type
        if pci_device.config().vendor_id() != 0x1af4 {
            return Err(VirtioAttachError::InvalidVendorId);
        }

        let port_base = match pci_device.config().bar(0) {
            Bar::IOMapped { port } => IoPort::new(port),
            Bar::MemoryMapped { .. } => {
                return Err(VirtioAttachError::NotSupportedBarType);
            }
        };

        pci_device.enable_bus_master();

        Ok(Arc::new(VirtioLegacyPci { port_base }))
    }
}

impl VirtioTransport for VirtioLegacyPci {
    fn is_modern(&self) -> bool {
        false
    }

    fn read_device_config8(&self, offset: u16) -> u8 {
        self.port_base.read8(REG_DEVICE_CONFIG_BASE + offset)
    }

    fn read_isr_status(&self) -> IsrStatus {
        IsrStatus::from_bits_truncate(self.port_base.read8(REG_ISR_STATUS))
    }

    fn read_device_status(&self) -> u8 {
        self.port_base.read8(REG_DEVICE_STATUS)
    }

    fn write_device_status(&self, value: u8) {
        self.port_base.write8(REG_DEVICE_STATUS, value);
        if value == 0 {
            while self.read_device_status() != 0 {
                trace!("vdev still not ready");
            }
        }
    }

    fn read_device_features(&self) -> u64 {
        self.port_base.read32(REG_DEVICE_FEATS) as u64
    }

    fn write_driver_features(&self, value: u64) {
        self.port_base.write32(REG_DRIVER_FEATS, value as u32);
    }

    fn select_queue(&self, index: u16) {
        self.port_base.write16(REG_QUEUE_SELECT, index);
    }

    fn queue_max_size(&self) -> u16 {
        self.port_base.read16(REG_QUEUE_SIZE)
    }

    fn set_queue_size(&self, _queue_size: u16) {
        // Nothing to do for a legacy device.
    }

    fn notify_queue(&self, index: u16) {
        self.port_base.write16(REG_QUEUE_NOTIFY, index);
    }

    fn enable_queue(&self) {
        // Nothing to do for a legacy device.
    }

    fn set_queue_desc_paddr(&self, paddr: PAddr) {
        self.port_base
            .write32(REG_QUEUE_ADDR_PFN, (paddr.value() / PAGE_SIZE) as u32);
    }

    fn set_queue_driver_paddr(&self, _paddr: PAddr) {
        // Nothing to do for a legacy device.
    }

    fn set_queue_device_paddr(&self, _paddr: PAddr) {
        // Nothing to do for a legacy device.
    }
}
