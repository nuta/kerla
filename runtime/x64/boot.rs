use super::{apic, bootinfo, cpu_local, gdt, idt, ioapic, pit, serial, syscall, tss, vga};
use crate::address::{PAddr, VAddr};
use crate::bootinfo::BootInfo;
use crate::logger;
use crate::page_allocator;

use x86::{
    controlregs::{self, Cr4, Xcr0},
    cpuid::CpuId,
    io::outb,
};

fn check_cpuid_feature(name: &str, supported: bool) {
    if !supported {
        panic!("{} is not supprted on this machine", name);
    }
}

/// Enables some CPU features.
unsafe fn common_setup(cpu_local_area: VAddr) {
    let feats = CpuId::new().get_feature_info().unwrap();
    let ex_feats = CpuId::new().get_extended_feature_info().unwrap();
    check_cpuid_feature("XSAVE", feats.has_xsave());
    check_cpuid_feature("FSGSBASE", ex_feats.has_fsgsbase());

    let mut cr4 = controlregs::cr4();
    cr4 |= Cr4::CR4_ENABLE_FSGSBASE
        | Cr4::CR4_ENABLE_OS_XSAVE
        | Cr4::CR4_ENABLE_SSE
        | Cr4::CR4_UNMASKED_SSE;
    controlregs::cr4_write(cr4);

    let mut xcr0 = controlregs::xcr0();
    xcr0 |= Xcr0::XCR0_SSE_STATE | Xcr0::XCR0_AVX_STATE;
    controlregs::xcr0_write(xcr0);

    cpu_local::init(cpu_local_area);
    apic::init();
    ioapic::init();
    gdt::init();
    tss::init();
    idt::init();
    pit::init();
    syscall::init();
}

/// Disables PIC. We use APIC instead.
unsafe fn init_pic() {
    outb(0xa1, 0xff);
    outb(0x21, 0xff);

    outb(0x20, 0x11);
    outb(0xa0, 0x11);
    outb(0x21, 0x20);
    outb(0xa1, 0x28);
    outb(0x21, 0x04);
    outb(0xa1, 0x02);
    outb(0x21, 0x01);
    outb(0xa1, 0x01);

    outb(0xa1, 0xff);
    outb(0x21, 0xff);
}

extern "Rust" {
    fn boot_kernel(bootinfo: &BootInfo) -> !;
}

/// Initializes the CPU. This function is called exactly once in the Bootstrap
/// Processor (BSP).
#[no_mangle]
unsafe extern "C" fn bsp_early_init(boot_magic: u32, boot_params: u64) -> ! {
    extern "C" {
        static __bsp_cpu_local: u8;
    }

    // Initialize the serial driver first to enable print macros.
    serial::early_init();
    vga::init();
    logger::init();

    let boot_info = bootinfo::parse(boot_magic, PAddr::new(boot_params as usize));
    page_allocator::init(&boot_info.ram_areas);

    logger::set_log_filter(&boot_info.log_filter);

    serial::init(boot_info.use_second_serialport);
    init_pic();
    common_setup(VAddr::new(&__bsp_cpu_local as *const _ as usize));

    boot_kernel(&boot_info);
}
