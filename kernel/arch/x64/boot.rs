use super::{address::PAddr, apic, gdt, idt, ioapic, multiboot, printchar, serial, syscall, tss};
use crate::boot::boot_kernel;
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
unsafe fn common_setup() {
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

    apic::init();
    ioapic::init();
    gdt::init();
    tss::init();
    idt::init();
    syscall::init();
}

/// Initializes the CPU. This function is called exactly once in the Bootstrap
/// Processor (BSP).
#[no_mangle]
unsafe extern "C" fn bsp_init(multiboot_magic: u32, multiboot_info: u64) -> ! {
    // Initialize the serial driver first to enable print macros.
    serial::init();
    printchar('\n');

    let boot_info = multiboot::parse(multiboot_magic, PAddr::new(multiboot_info));

    // Disables PIC -- we use IO APIC instead.
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

    common_setup();
    boot_kernel();
    unreachable!();
}
