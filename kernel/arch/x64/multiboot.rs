use super::address::PAddr;
use crate::boot::{BootInfo, RamArea};
use crate::utils::byte_size::ByteSize;
use arrayvec::ArrayVec;
use core::cmp::max;
use core::mem::size_of;

#[repr(u32)]
enum MultibootMagic {
    MultibootLegacy = 0x2badb002,
    Multiboot2 = 0x36d76289,
}

/// See https://www.gnu.org/software/grub/manual/multiboot/multiboot.html#Boot-information-format
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

extern "C" {
    static __kernel_image_end: u8;
}

unsafe fn parse_multiboot2_info(_info: *const u8) -> BootInfo {
    todo!();
}

unsafe fn parse_multiboot_legacy_info(info: &MultibootLegacyInfo) -> BootInfo {
    let mut off = 0;
    let mut ram_areas = ArrayVec::new();
    while off < info.memory_map_len {
        let entry: &MemoryMapEntry = &*PAddr::new((info.memory_map_addr + off) as usize).as_ptr();
        let type_name = match entry.entry_type {
            1 => {
                let image_end = &__kernel_image_end as *const _ as u64;
                let end = entry.base + entry.len;
                let base = max(entry.base, image_end);
                if image_end <= base && base < end {
                    ram_areas.push(RamArea {
                        base: PAddr::new(base as usize),
                        len: (end - base) as usize,
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

        println!(
            "multiboot: {:016x}-{:016x}  {}\t({})",
            entry.base,
            entry.base + entry.len,
            ByteSize::new(entry.len as usize),
            type_name,
        );

        off += entry.entry_size + size_of::<u32>() as u32;
    }

    BootInfo { ram_areas }
}

/// Parses a multiboot/multiboot2 boot information.
pub unsafe fn parse(magic: u32, info: PAddr) -> BootInfo {
    match magic {
        _ if magic == MultibootMagic::Multiboot2 as u32 => parse_multiboot2_info(info.as_ptr()),
        _ if magic == MultibootMagic::MultibootLegacy as u32 => {
            parse_multiboot_legacy_info(&*info.as_ptr())
        }
        _ => {
            panic!("invalid multiboot magic: {:x}", magic);
        }
    }
}
