use crate::address::{PAddr, VAddr};
use crate::bootinfo::{AllowedPciDevice, BootInfo, RamArea, VirtioMmioDevice};
use arrayvec::{ArrayString, ArrayVec};
use core::cmp::max;
use core::mem::size_of;
use core::slice;
use kerla_utils::alignment::align_up;
use kerla_utils::byte_size::ByteSize;

const MULTIBOOT_MAGIC_LEGACY: u32 = 0x2badb002;
const MULTIBOOT_MAGIC_2: u32 = 0x36d76289;
const LINUXBOOT_MAGIC: u32 = 0xb002b002;

/// See <https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format>
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct Multiboot2InfoHeader {
    total_size: u32,
    reserved: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct Multiboot2TagHeader {
    tag_type: u32,
    size: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct Multiboot2MemoryMapTag {
    tag_type: u32,
    tag_size: u32,
    entry_size: u32,
    entry_version: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct Multiboot2MemoryMapEntry {
    base: u64,
    len: u64,
    entry_type: u32,
    reserved: u32,
}
/// See <https://www.gnu.org/software/grub/manual/multiboot/multiboot.html#Boot-information-format>
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct MultibootLegacyInfo {
    flags: u32,
    mem_lower: u32,
    mem_upper: u32,
    boot_device: u32,
    cmdline: u32,
    mods_count: u32,
    mods_addr: u32,
    syms: [u8; 16],
    memory_map_len: u32,
    memory_map_addr: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct MemoryMapEntry {
    entry_size: u32,
    base: u64,
    len: u64,
    entry_type: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct E820Entry {
    addr: u64,
    size: u64,
    entry_type: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct SetupHeader {
    setup_sects: u8,
    root_flags: u16,
    syssize: u32,
    ram_size: u16,
    vid_mode: u16,
    root_dev: u16,
    boot_flag: u16,
    jump: u16,
    header: u32,
    version: u16,
    realmode_swtch: u32,
    start_sys_seg: u16,
    kernel_version: u16,
    type_of_loader: u8,
    loadflags: u8,
    setup_move_size: u16,
    code32_start: u32,
    ramdisk_image: u32,
    ramdisk_size: u32,
    bootsect_kludge: u32,
    heap_end_ptr: u16,
    ext_loader_ver: u8,
    ext_loader_type: u8,
    cmd_line_ptr: u32,
    initrd_addr_max: u32,
    kernel_alignment: u32,
    relocatable_kernel: u8,
    min_alignment: u8,
    xloadflags: u16,
    cmdline_size: u32,
    hardware_subarch: u32,
    hardware_subarch_data: u64,
    payload_offset: u32,
    payload_length: u32,
    setup_data: u64,
    pref_address: u64,
    init_size: u32,
    handover_offset: u32,
    kernel_info_offset: u32,
}

extern "C" {
    static __kernel_image_end: u8;
}

struct Cmdline {
    pub pci_enabled: bool,
    pub virtio_mmio_devices: ArrayVec<VirtioMmioDevice, 4>,
    pub log_filter: ArrayString<64>,
    pub use_second_serialport: bool,
    pub dhcp_enabled: bool,
    pub ip4: Option<ArrayString<18>>,
    pub gateway_ip4: Option<ArrayString<15>>,
    pub pci_allowlist: ArrayVec<AllowedPciDevice, 4>,
}

impl Cmdline {
    pub fn parse(cmdline: &[u8]) -> Cmdline {
        let s = core::str::from_utf8(cmdline).expect("cmdline is not a utf-8 string");
        info!("cmdline: {}", if s.is_empty() { "(empty)" } else { s });

        let mut pci_enabled = true;
        let mut pci_allowlist = ArrayVec::new();
        let mut virtio_mmio_devices = ArrayVec::new();
        let mut log_filter = ArrayString::new();
        let mut use_second_serialport = false;
        let mut dhcp_enabled = true;
        let mut ip4 = None;
        let mut gateway_ip4 = None;
        if !s.is_empty() {
            for config in s.split(' ') {
                if config.is_empty() {
                    continue;
                }

                let mut words = config.splitn(2, '=');
                match (words.next(), words.next()) {
                    (Some("pci"), Some("off")) => {
                        warn!("bootinfo: PCI disabled");
                        pci_enabled = false;
                    }
                    (Some("pci_device"), Some(bus_and_slot)) => {
                        warn!("bootinfo: allowed PCI device: {}", bus_and_slot);
                        let mut iter = bus_and_slot.splitn(2, ':');
                        let bus = iter
                            .next()
                            .and_then(|w| w.parse().ok())
                            .expect("bootinfo.bus_and_slot must be formed as bus:slot");
                        let slot = iter
                            .next()
                            .and_then(|w| w.parse().ok())
                            .expect("bootinfo.bus_and_slot must be formed as bus:slot");
                        pci_allowlist.push(AllowedPciDevice { bus, slot });
                    }
                    (Some("serial1"), Some("on")) => {
                        info!("bootinfo: secondary serial port enabled");
                        use_second_serialport = true;
                    }
                    (Some("log"), Some(value)) => {
                        info!("bootinfo: log filter = \"{}\"", value);
                        if log_filter.try_push_str(value).is_err() {
                            warn!("bootinfo: log filter is too long");
                        }
                    }
                    (Some("virtio_mmio.device"), Some(value)) => {
                        let (_size, rest) = value.split_once("@0x").unwrap();
                        let (addr, irq) = rest.split_once(':').unwrap();
                        let addr = usize::from_str_radix(addr, 16).unwrap();
                        let irq = irq.parse().unwrap();

                        info!(
                            "bootinfo: virtio MMIO device: base={:016x}, irq={}",
                            addr, irq
                        );
                        virtio_mmio_devices.push(VirtioMmioDevice {
                            mmio_base: PAddr::new(addr),
                            irq,
                        })
                    }
                    (Some("dhcp"), Some("off")) => {
                        warn!("bootinfo: DHCP disabled");
                        dhcp_enabled = false;
                    }
                    (Some("ip4"), Some(value)) => {
                        let mut s = ArrayString::new();
                        if s.try_push_str(value).is_err() {
                            warn!("bootinfo: ip4 is too long");
                        }
                        ip4 = Some(s);
                    }
                    (Some("gateway_ip4"), Some(value)) => {
                        let mut s = ArrayString::new();
                        if s.try_push_str(value).is_err() {
                            warn!("bootinfo: gateway_ip4 is too long");
                        }
                        gateway_ip4 = Some(s);
                    }
                    (Some(path), None) if path.starts_with('/') => {
                        // QEMU appends a kernel image path. Just ignore it.
                    }
                    _ => {
                        warn!("cmdline: unsupported option, ignoring: '{}'", config);
                    }
                }
            }
        }

        Cmdline {
            pci_enabled,
            pci_allowlist,
            virtio_mmio_devices,
            log_filter,
            use_second_serialport,
            dhcp_enabled,
            ip4,
            gateway_ip4,
        }
    }
}

fn process_memory_map_entry(
    ram_areas: &mut ArrayVec<RamArea, 8>,
    entry_type: u32,
    base: usize,
    len: usize,
) {
    let type_name = match entry_type {
        1 => {
            let image_end = unsafe { &__kernel_image_end as *const _ as usize };
            let end = base + len;
            let base = max(base, image_end);
            if image_end <= base && base < end {
                ram_areas.push(RamArea {
                    base: PAddr::new(base),
                    len: end - base,
                });
            }

            "available RAM"
        }
        2 => "reserved",
        3 => "ACPI",
        4 => "NVS",
        5 => "defective",
        _ => "unknown",
    };

    trace!(
        "{:>14}: {:016x}-{:016x} {}",
        type_name,
        base,
        base + len,
        ByteSize::new(len),
    );
}

unsafe fn parse_multiboot2_info(header: &Multiboot2InfoHeader) -> BootInfo {
    let header_vaddr = VAddr::new(header as *const _ as usize);
    let mut off = size_of::<Multiboot2TagHeader>();
    let mut ram_areas = ArrayVec::new();
    let mut cmdline = None;
    while off + size_of::<Multiboot2TagHeader>() < header.total_size as usize {
        let tag_vaddr = header_vaddr.add(off);
        let tag = &*tag_vaddr.as_ptr::<Multiboot2TagHeader>();
        match tag.tag_type {
            1 => {
                // Command line.
                let cstr = tag_vaddr
                    .add(size_of::<Multiboot2TagHeader>())
                    .as_ptr::<u8>();
                let mut len = 0;
                while cstr.add(len).read() != 0 {
                    len += 1;
                }

                cmdline = Some(
                    core::str::from_utf8(slice::from_raw_parts(cstr, len))
                        .expect("cmdline is not a utf-8 string"),
                );
            }
            6 => {
                // Memory map.
                let tag = &*(tag as *const Multiboot2TagHeader as *const Multiboot2MemoryMapTag);
                let mut entry_off = size_of::<Multiboot2MemoryMapTag>();
                while entry_off < tag.tag_size as usize {
                    let entry = &*tag_vaddr
                        .add(entry_off)
                        .as_ptr::<Multiboot2MemoryMapEntry>();

                    process_memory_map_entry(
                        &mut ram_areas,
                        entry.entry_type,
                        entry.base as usize,
                        entry.len as usize,
                    );

                    entry_off += tag.entry_size as usize;
                }
            }
            _ => {
                // Unsupported tag. Ignored .
            }
        }

        off = align_up(off + tag.size as usize, 8);
    }

    assert!(!ram_areas.is_empty());
    let cmdline = Cmdline::parse(cmdline.unwrap_or("").as_bytes());
    BootInfo {
        ram_areas,
        pci_enabled: cmdline.pci_enabled,
        pci_allowlist: cmdline.pci_allowlist,
        virtio_mmio_devices: cmdline.virtio_mmio_devices,
        log_filter: cmdline.log_filter,
        use_second_serialport: cmdline.use_second_serialport,
        dhcp_enabled: cmdline.dhcp_enabled,
        ip4: cmdline.ip4,
        gateway_ip4: cmdline.gateway_ip4,
    }
}

unsafe fn parse_multiboot_legacy_info(info: &MultibootLegacyInfo) -> BootInfo {
    let mut off = 0;
    let mut ram_areas = ArrayVec::new();
    while off < info.memory_map_len {
        let entry: &MemoryMapEntry = &*PAddr::new((info.memory_map_addr + off) as usize).as_ptr();
        process_memory_map_entry(
            &mut ram_areas,
            entry.entry_type,
            entry.base as usize,
            entry.len as usize,
        );

        off += entry.entry_size + size_of::<u32>() as u32;
    }

    let mut cmdline = None;
    if info.cmdline != 0 {
        // Command line.
        let cstr = PAddr::new(info.cmdline as usize).as_ptr::<u8>();
        let mut len = 0;
        while cstr.add(len).read() != 0 {
            len += 1;
        }

        cmdline = Some(
            core::str::from_utf8(slice::from_raw_parts(cstr, len))
                .expect("cmdline is not a utf-8 string"),
        );
        trace!("cmdline={:?}", cmdline);
    }

    let cmdline = Cmdline::parse(cmdline.unwrap_or("").as_bytes());
    BootInfo {
        ram_areas,
        pci_enabled: cmdline.pci_enabled,
        pci_allowlist: cmdline.pci_allowlist,
        virtio_mmio_devices: cmdline.virtio_mmio_devices,
        log_filter: cmdline.log_filter,
        use_second_serialport: cmdline.use_second_serialport,
        dhcp_enabled: cmdline.dhcp_enabled,
        ip4: cmdline.ip4,
        gateway_ip4: cmdline.gateway_ip4,
    }
}

unsafe fn parse_linux_boot_params(boot_params: PAddr) -> BootInfo {
    let e820_entries = *boot_params.add(0x1e8).as_ptr();
    let setup_header: &SetupHeader = &*boot_params.add(0x1f1).as_ptr();
    let e820_table: &[E820Entry; 128] = &*boot_params.add(0x2d0).as_ptr();

    let mut ram_areas = ArrayVec::new();
    for i in 0..e820_entries {
        let entry = &e820_table[i as usize];
        process_memory_map_entry(
            &mut ram_areas,
            entry.entry_type,
            entry.addr as usize,
            entry.size as usize,
        );
    }

    let cmdline = Cmdline::parse(core::slice::from_raw_parts(
        setup_header.cmd_line_ptr as *const u8,
        setup_header
            .cmdline_size
            .saturating_sub(1 /* trailing NUL */) as usize,
    ));
    BootInfo {
        ram_areas,
        pci_enabled: cmdline.pci_enabled,
        pci_allowlist: cmdline.pci_allowlist,
        virtio_mmio_devices: cmdline.virtio_mmio_devices,
        log_filter: cmdline.log_filter,
        use_second_serialport: cmdline.use_second_serialport,
        dhcp_enabled: cmdline.dhcp_enabled,
        ip4: cmdline.ip4,
        gateway_ip4: cmdline.gateway_ip4,
    }
}

/// Parses a multiboot/multiboot2/linux boot protocol boot information.
pub unsafe fn parse(magic: u32, info: PAddr) -> BootInfo {
    match magic {
        MULTIBOOT_MAGIC_2 => parse_multiboot2_info(&*info.as_ptr()),
        MULTIBOOT_MAGIC_LEGACY => parse_multiboot_legacy_info(&*info.as_ptr()),
        LINUXBOOT_MAGIC => parse_linux_boot_params(info),
        _ => {
            panic!("invalid boot magic: {:x}", magic);
        }
    }
}
