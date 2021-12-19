use crate::device::IsrStatus;

use core::convert::TryInto;

use alloc::sync::Arc;
use kerla_api::{
    address::{PAddr, VAddr},
    driver::pci::{Bar, PciCapability, PciDevice},
};
use memoffset::offset_of;

use super::{VirtioAttachError, VirtioTransport};

const VIRTIO_F_VERSION_1: u64 = 1 << 32;

const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

/// Walk capabilities list. A capability consists of the following fields
/// (from "4.1.4 Virtio Structure PCI Capabilities"):
///
/// ```text
/// struct virtio_pci_cap {
///     u8 cap_vndr;    /* Generic PCI field: PCI_CAP_ID_VNDR */
///     u8 cap_next;    /* Generic PCI field: next ptr. */
///     u8 cap_len;     /* Generic PCI field: capability length */
///     u8 cfg_type;    /* Identifies the structure. */
///     u8 bar;         /* Where to find it. */
///     u8 padding[3];  /* Pad to full dword. */
///     le32 offset;    /* Offset within bar. */
///     le32 length;    /* Length of the structure, in bytes. */
/// };
/// ```
fn get_bar_for_cfg_type(pci_device: &PciDevice, cfg_type: u8) -> Option<(&PciCapability, VAddr)> {
    let cap = pci_device
        .capabilities()
        .iter()
        .find(|cap| cap.id == 9 && cap.data.len() >= 16 && cap.data[3] == cfg_type)?;

    let offset = u32::from_le_bytes(cap.data[8..12].try_into().unwrap()) as usize;
    match pci_device.config().bar(cap.data[4] as usize) {
        Bar::MemoryMapped { paddr } => Some((cap, paddr.as_vaddr().add(offset))),
        _ => {
            warn!("virtio-pci only supports memory-mapped I/O access for now");
            None
        }
    }
}

pub struct VirtioModernPci {
    common_cfg: VAddr,
    device_cfg: VAddr,
    notify: VAddr,
    notify_off_multiplier: u32,
    isr: VAddr,
}

impl VirtioModernPci {
    pub fn probe_pci(
        pci_device: &PciDevice,
    ) -> Result<Arc<dyn VirtioTransport>, VirtioAttachError> {
        // TODO: Check device type
        if pci_device.config().vendor_id() != 0x1af4 {
            return Err(VirtioAttachError::InvalidVendorId);
        }

        let common_cfg = get_bar_for_cfg_type(pci_device, VIRTIO_PCI_CAP_COMMON_CFG)
            .ok_or(VirtioAttachError::MissingPciCommonCfg)?
            .1;
        let device_cfg = get_bar_for_cfg_type(pci_device, VIRTIO_PCI_CAP_DEVICE_CFG)
            .ok_or(VirtioAttachError::MissingPciDeviceCfg)?
            .1;
        let isr = get_bar_for_cfg_type(pci_device, VIRTIO_PCI_CAP_ISR_CFG)
            .ok_or(VirtioAttachError::MissingPciIsrCfg)?
            .1;

        let (notify, notify_off_multiplier) =
            get_bar_for_cfg_type(pci_device, VIRTIO_PCI_CAP_NOTIFY_CFG)
                .map(|(cap, mmio)| {
                    // struct virtio_pci_notify_cap {
                    //     struct virtio_pci_cap cap;
                    //     le32 notify_off_multiplier; /* Multiplier for queue_notify_off. */
                    // };
                    let notify_off_multiplier =
                        u32::from_le_bytes(cap.data[16..20].try_into().unwrap());
                    (mmio, notify_off_multiplier)
                })
                .ok_or(VirtioAttachError::MissingPciNotifyCfg)?;

        pci_device.enable_bus_master();

        Ok(Arc::new(VirtioModernPci {
            common_cfg,
            device_cfg,
            notify,
            notify_off_multiplier,
            isr,
        }))
    }
}

#[repr(C, packed)]
struct CommonCfg {
    device_feature_select: u32,
    device_feature: u32,
    driver_feature_select: u32,
    driver_feature: u32,
    msix_config: u16,
    num_queues: u16,
    device_status: u8,
    config_generation: u8,
    queue_select: u16,
    queue_size: u16,
    queue_msix_vector: u16,
    queue_enable: u16,
    queue_notify_off: u16,
    queue_desc_lo: u32,
    queue_desc_hi: u32,
    queue_driver_lo: u32,
    queue_driver_hi: u32,
    queue_device_lo: u32,
    queue_device_hi: u32,
}

impl VirtioTransport for VirtioModernPci {
    fn is_modern(&self) -> bool {
        true
    }

    fn read_device_config8(&self, offset: u16) -> u8 {
        unsafe { self.device_cfg.add(offset as usize).read_volatile::<u8>() }
    }

    fn read_isr_status(&self) -> IsrStatus {
        IsrStatus::from_bits_truncate(unsafe { self.isr.add(0).read_volatile::<u8>() })
    }

    fn read_device_status(&self) -> u8 {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, device_status))
                .read_volatile::<u8>()
        }
    }

    fn write_device_status(&self, value: u8) {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, device_status))
                .write_volatile::<u8>(value)
        }
    }

    fn read_device_features(&self) -> u64 {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, device_feature_select))
                .write_volatile::<u32>(0);
            let low = self
                .common_cfg
                .add(offset_of!(CommonCfg, device_feature))
                .read_volatile::<u32>();

            self.common_cfg
                .add(offset_of!(CommonCfg, device_feature_select))
                .write_volatile::<u32>(1);
            let high = self
                .common_cfg
                .add(offset_of!(CommonCfg, device_feature))
                .read_volatile::<u32>();

            ((high as u64) << 32) | (low as u64)
        }
    }

    fn write_driver_features(&self, mut value: u64) {
        value |= VIRTIO_F_VERSION_1;

        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, driver_feature_select))
                .write_volatile::<u32>(0);
            self.common_cfg
                .add(offset_of!(CommonCfg, driver_feature))
                .write_volatile::<u32>((value & 0xffff_ffff) as u32);

            self.common_cfg
                .add(offset_of!(CommonCfg, driver_feature_select))
                .write_volatile::<u32>(1);
            self.common_cfg
                .add(offset_of!(CommonCfg, driver_feature))
                .write_volatile::<u32>((value >> 32) as u32);
        }
    }

    fn select_queue(&self, index: u16) {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_select))
                .write_volatile::<u16>(index)
        }
    }

    fn queue_max_size(&self) -> u16 {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_size))
                .read_volatile::<u16>()
        }
    }

    fn set_queue_size(&self, queue_size: u16) {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_size))
                .write_volatile::<u16>(queue_size)
        }
    }

    fn notify_queue(&self, index: u16) {
        unsafe {
            let offset = self.notify_off_multiplier
                * self
                    .common_cfg
                    .add(offset_of!(CommonCfg, queue_notify_off))
                    .read_volatile::<u16>() as u32;
            self.notify
                .add(offset as usize)
                .write_volatile::<u16>(index)
        }
    }

    fn enable_queue(&self) {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_enable))
                .write_volatile::<u16>(1);
        }
    }

    fn set_queue_desc_paddr(&self, paddr: PAddr) {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_desc_lo))
                .write_volatile::<u32>((paddr.value() & 0xffff_ffff) as u32);
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_desc_hi))
                .write_volatile::<u32>((paddr.value() >> 32) as u32);
        }
    }

    fn set_queue_driver_paddr(&self, paddr: PAddr) {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_driver_lo))
                .write_volatile::<u32>((paddr.value() & 0xffff_ffff) as u32);
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_driver_hi))
                .write_volatile::<u32>((paddr.value() >> 32) as u32);
        }
    }

    fn set_queue_device_paddr(&self, paddr: PAddr) {
        unsafe {
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_device_lo))
                .write_volatile::<u32>((paddr.value() & 0xffff_ffff) as u32);
            self.common_cfg
                .add(offset_of!(CommonCfg, queue_device_hi))
                .write_volatile::<u32>((paddr.value() >> 32) as u32);
        }
    }
}
