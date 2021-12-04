use arrayvec::{ArrayString, ArrayVec};

use crate::address::PAddr;

pub struct RamArea {
    pub base: PAddr,
    pub len: usize,
}

pub struct VirtioMmioDevice {
    pub mmio_base: PAddr,
    pub irq: u8,
}

pub struct AllowedPciDevice {
    pub bus: u8,
    pub slot: u8,
}

pub struct BootInfo {
    pub ram_areas: ArrayVec<RamArea, 8>,
    pub virtio_mmio_devices: ArrayVec<VirtioMmioDevice, 4>,
    pub log_filter: ArrayString<64>,
    pub pci_enabled: bool,
    pub pci_allowlist: ArrayVec<AllowedPciDevice, 4>,
    pub use_second_serialport: bool,
    pub dhcp_enabled: bool,
    pub ip4: Option<ArrayString<18>>,
    pub gateway_ip4: Option<ArrayString<15>>,
}
