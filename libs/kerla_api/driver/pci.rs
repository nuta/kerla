use alloc::vec::Vec;
use arrayvec::ArrayVec;
use core::convert::TryInto;
use core::{mem::size_of, mem::MaybeUninit};
use kerla_runtime::address::PAddr;
use kerla_utils::alignment::is_aligned;
use x86::io::{inl, outl};

const PCI_IOPORT_ADDR: u16 = 0x0cf8;
const PCI_IOPORT_DATA: u16 = 0x0cf8 + 0x04;

pub type VendorId = u16;
pub type DeviceId = u16;

/// PCI configuration space. We only use the one such that header_type == 0x00.
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct PciConfig {
    vendor_id: VendorId,
    device_id: DeviceId,
    command: u16,
    status: u16,
    revision: u8,
    prog_if: u8,
    subclass: u8,
    class: u8,
    cache_line_size: u8,
    latency_timer: u8,
    header_type: u8,
    bist: u8,
    bar: [u32; 6],
    cardbus_cis_ptr: u32,
    subsystem_vendor: u16,
    subsystem: u16,
    rom_base: u32,
    capabilities_ptr: u8,
    reserved0: [u8; 3],
    reserved1: u32,
    interrupt_line: u8,
    interrupt_pin: u8,
    min_grant: u8,
    max_latency: u8,
}

#[derive(Debug)]
pub enum Bar {
    IOMapped { port: u16 },
    MemoryMapped { paddr: PAddr },
}

#[derive(Debug)]
pub struct PciCapability {
    pub id: u8,
    pub data: Vec<u8>,
}

impl PciConfig {
    pub fn bar(&self, index: usize) -> Bar {
        assert!(index < 5);
        let bar = self.bar[index];
        if bar & 1 == 0 {
            Bar::MemoryMapped {
                paddr: PAddr::new((bar & !0b1111) as usize),
            }
        } else {
            Bar::IOMapped {
                port: (bar & !0b1111).try_into().unwrap(),
            }
        }
    }

    pub fn vendor_id(&self) -> VendorId {
        self.vendor_id
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn interrupt_line(&self) -> u8 {
        self.interrupt_line
    }
}

macro_rules! pci_config_offset {
    ($field:tt) => {
        ::memoffset::offset_of!(PciConfig, $field) as u32
    };
}

pub struct PciDevice {
    config: PciConfig,
    bus: u8,
    slot: u8,
    capabilities: ArrayVec<PciCapability, 16>,
}

impl PciDevice {
    pub fn config(&self) -> &PciConfig {
        &self.config
    }

    pub fn enable_bus_master(&self) {
        let bus = PciBus {};
        let value = bus.read32(self.bus, self.slot, pci_config_offset!(command));
        bus.write32(
            self.bus,
            self.slot,
            pci_config_offset!(command),
            value | (1 << 2),
        );
    }

    pub fn capabilities(&self) -> &[PciCapability] {
        &self.capabilities
    }
}

#[derive(Copy, Clone)]
struct PciBus {}

impl PciBus {
    pub fn new() -> PciBus {
        PciBus {}
    }

    pub fn read32(&self, bus: u8, slot: u8, offset: u32) -> u32 {
        assert!(is_aligned(offset as usize, 4));
        let addr = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | offset;
        unsafe {
            outl(PCI_IOPORT_ADDR, addr);
            inl(PCI_IOPORT_DATA)
        }
    }

    pub fn read8(&self, bus: u8, slot: u8, offset: u32) -> u8 {
        let value = self.read32(bus, slot, offset & 0xfffc);
        ((value >> ((offset & 0x03) * 8)) & 0xff) as u8
    }

    pub fn write32(&self, bus: u8, slot: u8, offset: u32, value: u32) {
        assert!(is_aligned(offset as usize, 4));
        let addr = (1 << 31) | ((bus as u32) << 16) | ((slot as u32) << 11) | offset;
        unsafe {
            outl(PCI_IOPORT_ADDR, addr);
            outl(PCI_IOPORT_DATA, value);
        }
    }

    pub fn read_device_config(&self, bus: u8, slot: u8) -> Option<PciConfig> {
        if self.read32(bus, slot, pci_config_offset!(vendor_id)) == 0xffff {
            return None;
        }

        let header_type = self.read8(bus, slot, pci_config_offset!(header_type));
        if header_type != 0 {
            return None;
        }

        let mut config = MaybeUninit::uninit();
        for i in 0..(size_of::<PciConfig>() / size_of::<u32>()) {
            unsafe {
                *(config.as_mut_ptr() as *mut u32).add(i) =
                    self.read32(bus, slot, (i * size_of::<u32>()) as u32);
            }
        }

        Some(unsafe { config.assume_init() })
    }

    pub fn read_capabilities(&self, bus: u8, slot: u8) -> ArrayVec<PciCapability, 16> {
        let mut caps = ArrayVec::new();

        let mut cap_off = self.read8(bus, slot, 0x34) as u32;
        while cap_off != 0 {
            let id = self.read8(bus, slot, cap_off);
            let next_off = self.read8(bus, slot, cap_off + 1);
            let len = self.read8(bus, slot, cap_off + 2);
            let mut data = Vec::with_capacity(len as usize);
            for i in 0..len {
                data.push(self.read8(bus, slot, cap_off + i as u32));
            }

            caps.push(PciCapability { id, data });
            cap_off = next_off as u32;
        }

        caps
    }
}

pub struct PciScanner {
    bus: PciBus,
    bus_no: u8,
    slot: u8,
}

/// Enumerates all PCI devices.
pub fn enumerate_pci_devices() -> PciScanner {
    PciScanner {
        bus: PciBus::new(),
        bus_no: 0,
        slot: 0,
    }
}

impl Iterator for PciScanner {
    type Item = PciDevice;
    fn next(&mut self) -> Option<Self::Item> {
        while !(self.bus_no == 255 && self.slot == 31) {
            if self.slot == 31 {
                self.bus_no += 1;
                self.slot = 0;
            }

            let config = self.bus.read_device_config(self.bus_no, self.slot);
            self.slot += 1;

            if let Some(config) = config {
                let capabilities = self.bus.read_capabilities(self.bus_no, self.slot - 1);
                return Some(PciDevice {
                    bus: self.bus_no,
                    slot: self.slot - 1,
                    config,
                    capabilities,
                });
            }
        }
        None
    }
}
