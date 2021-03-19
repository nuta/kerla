use crate::{arch::PageTable, arch::UserVAddr, arch::VAddr, fs::inode::FileLike};
use alloc::sync::Arc;
use alloc::vec::Vec;

pub enum VmAreaType {
    Anonymous,
    File {
        file: Arc<dyn FileLike>,
        offset: usize,
    },
}

pub struct VmArea {
    start: UserVAddr,
    len: usize,
    area_type: VmAreaType,
}

impl VmArea {
    pub fn area_type(&self) -> &VmAreaType {
        &self.area_type
    }

    pub fn offset_in_vma(&self, vaddr: UserVAddr) -> usize {
        debug_assert!(self.contains(vaddr));
        vaddr.value() - self.start.value()
    }

    pub fn contains(&self, vaddr: UserVAddr) -> bool {
        self.start.value() <= vaddr.value() && vaddr.value() < self.start.value() + self.len
    }
}

pub struct Vm {
    page_table: PageTable,
    vm_areas: Vec<VmArea>,
}

impl Vm {
    pub fn new() -> Vm {
        Vm {
            page_table: PageTable::new(),
            vm_areas: Vec::new(),
        }
    }

    pub fn page_table(&self) -> &PageTable {
        &self.page_table
    }

    pub fn page_table_mut(&mut self) -> &mut PageTable {
        &mut self.page_table
    }

    pub fn vm_areas(&self) -> &[VmArea] {
        &self.vm_areas
    }

    pub fn add_vm_area(&mut self, start: UserVAddr, len: usize, area_type: VmAreaType) {
        self.vm_areas.push(VmArea {
            start,
            len,
            area_type,
        });
    }
}
