use super::address::{PAddr, VAddr};
use crate::boot::{BootInfo, RamArea};
use arrayvec::ArrayVec;
use core::cmp::max;
use core::mem::size_of;
use penguin_utils::alignment::align_up;
use penguin_utils::byte_size::ByteSize;

#[repr(u32)]
enum MultibootMagic {
    MultibootLegacy = 0x2badb002,
    Multiboot2 = 0x36d76289,
}

/// See https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html#Boot-information-format
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
        "multiboot2: {:016x}-{:016x}  {}\t({})",
        base,
        base + len,
        ByteSize::new(len),
        type_name,
    );
}

unsafe fn parse_multiboot2_info(header: &Multiboot2InfoHeader) -> BootInfo {
    let header_vaddr = VAddr::new(header as *const _ as usize);
    let mut off = size_of::<Multiboot2TagHeader>();
    let mut ram_areas = ArrayVec::new();
    while off + size_of::<Multiboot2TagHeader>() < header.total_size as usize {
        let tag_vaddr = header_vaddr.add(off);
        let tag = &*tag_vaddr.as_ptr::<Multiboot2TagHeader>();
        if tag.tag_type == 6 {
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

        off = align_up(off + tag.size as usize, 8);
    }

    assert!(!ram_areas.is_empty());
    BootInfo { ram_areas }
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

    BootInfo { ram_areas }
}

/// Parses a multiboot/multiboot2 boot information.
pub unsafe fn parse(magic: u32, info: PAddr) -> BootInfo {
    match magic {
        _ if magic == MultibootMagic::Multiboot2 as u32 => parse_multiboot2_info(&*info.as_ptr()),
        _ if magic == MultibootMagic::MultibootLegacy as u32 => {
            parse_multiboot_legacy_info(&*info.as_ptr())
        }
        _ => {
            panic!("invalid multiboot magic: {:x}", magic);
        }
    }
}
