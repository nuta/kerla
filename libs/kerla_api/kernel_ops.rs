//! Internal APIs exposed for kerla_kernel crate. **Don't use from your kernel extensions!**
use alloc::boxed::Box;
use kerla_runtime::bootinfo::VirtioMmioDevice;
use kerla_utils::static_cell::StaticCell;

use crate::driver::{self, net::EthernetDriver};

pub trait KernelOps: Sync {
    fn receive_etherframe_packet(&self, pkt: &[u8]);
    fn register_ethernet_driver(&self, driver: Box<dyn EthernetDriver>);
    fn attach_irq(&self, irq: u8, f: Box<dyn FnMut() + Send + Sync + 'static>);
}

static OPS: StaticCell<&dyn KernelOps> = StaticCell::new(&NopOps);

struct NopOps;

impl KernelOps for NopOps {
    fn attach_irq(&self, _irq: u8, _f: Box<dyn FnMut() + Send + Sync + 'static>) {}
    fn register_ethernet_driver(&self, _driver: Box<dyn EthernetDriver>) {}
    fn receive_etherframe_packet(&self, _pkt: &[u8]) {}
}

pub(crate) fn kernel_ops() -> &'static dyn KernelOps {
    OPS.load()
}

pub fn set_kernel_ops(ops: &'static dyn KernelOps) {
    OPS.store(ops);
}

pub fn init(ops: &'static dyn KernelOps) {
    set_kernel_ops(ops);
}

pub fn init_drivers(pci_enabled: bool, mmio_devices: &[VirtioMmioDevice]) {
    driver::init(pci_enabled, mmio_devices);
}
