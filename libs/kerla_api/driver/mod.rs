//! Device driver APIs.
use crate::kernel_ops::kernel_ops;

use alloc::vec::Vec;

pub mod ioport;
pub mod net;
pub mod pci;

pub use kerla_runtime::bootinfo::VirtioMmioDevice;

use alloc::boxed::Box;
use kerla_runtime::spinlock::SpinLock;

use self::pci::PciDevice;

static DEVICE_PROBERS: SpinLock<Vec<Box<dyn DeviceProber>>> = SpinLock::new(Vec::new());

pub trait DeviceProber: Send + Sync {
    fn probe_pci(&self, pci_device: &PciDevice);
    fn probe_virtio_mmio(&self, mmio_device: &VirtioMmioDevice);
}

pub fn register_driver_prober(driver: Box<dyn DeviceProber>) {
    DEVICE_PROBERS.lock().push(driver);
}

pub fn attach_irq<F: FnMut() + Send + Sync + 'static>(irq: u8, f: F) {
    kernel_ops().attach_irq(irq, Box::new(f))
}

pub fn init(pci_enabled: bool, mmio_devices: &[VirtioMmioDevice]) {
    // Scan PCI devices.
    if pci_enabled {
        for device in pci::enumerate_pci_devices() {
            trace!(
                "pci: found a device: id={:04x}:{:04x}, bar0={:016x?}, irq={}",
                device.config().vendor_id(),
                device.config().device_id(),
                device.config().bar(0),
                device.config().interrupt_line()
            );

            for prober in DEVICE_PROBERS.lock().iter() {
                prober.probe_pci(&device);
            }
        }
    }

    // Register Virtio devices connected over MMIO.
    for device in mmio_devices {
        for prober in DEVICE_PROBERS.lock().iter() {
            prober.probe_virtio_mmio(device);
        }
    }
}
