use super::{
    address::{PAddr, VAddr},
    apic, gdt, idt, ioapic, multiboot, printchar, serial, syscall, tss,
};
use crate::boot::boot_kernel;
use core::ptr;
use x86::{
    bits64::segmentation::wrgsbase,
    controlregs::{self, Cr4, Xcr0},
    cpuid::CpuId,
    io::outb,
};

fn check_cpuid_feature(name: &str, supported: bool) {
    if !supported {
        panic!("{} is not supprted on this machine", name);
    }
}

unsafe fn init_cpu_local(cpu_local_area: VAddr) {
    extern "C" {
        static __cpu_local: u8;
        static __cpu_local_size: u8;
    }

    let template = VAddr::new(&__cpu_local as *const _ as u64);
    let len = &__cpu_local_size as *const _ as usize;
    ptr::copy_nonoverlapping::<u8>(template.as_ptr(), cpu_local_area.as_mut_ptr(), len);

    wrgsbase(cpu_local_area.value() as u64);
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

    init_cpu_local(cpu_local_area);

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
    extern "C" {
        static __bsp_cpu_local: u8;
    }

    // Initialize the serial driver first to enable print macros.
    serial::init();
    printchar('\n');

    let _boot_info = multiboot::parse(multiboot_magic, PAddr::new(multiboot_info));

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

    common_setup(VAddr::new(&__bsp_cpu_local as *const _ as u64));
    boot_kernel();
    unreachable!();
}
