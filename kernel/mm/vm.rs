use crate::{arch::PageTable, arch::UserVAddr, arch::PAGE_SIZE, fs::inode::FileLike};
use crate::{
    arch::USER_STACK_TOP,
    result::{Errno, Error, Result},
};
use alloc::sync::Arc;
use alloc::vec::Vec;
use penguin_utils::alignment::{align_up, is_aligned};

pub enum VmAreaType {
    Anonymous,
    File {
        file: Arc<dyn FileLike>,
        offset: usize,
        file_size: usize,
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

    pub fn start(&self) -> UserVAddr {
        self.start
    }

    pub fn end(&self) -> UserVAddr {
        self.start.add(self.len).unwrap()
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
    pub fn new(stack_bottom: UserVAddr, heap_bottom: UserVAddr) -> Vm {
        debug_assert!(is_aligned(stack_bottom.value(), PAGE_SIZE));
        debug_assert!(is_aligned(heap_bottom.value(), PAGE_SIZE));

        let stack_vma = VmArea {
            start: stack_bottom,
            len: USER_STACK_TOP.value() - stack_bottom.value(),
            area_type: VmAreaType::Anonymous,
        };

        let heap_vma = VmArea {
            start: heap_bottom,
            len: 0,
            area_type: VmAreaType::Anonymous,
        };

        Vm {
            page_table: PageTable::new(),
            // The order of elements must be unchanged because `stack_vma_mut()`
            // and `heap_vma_mut` depends on it.
            vm_areas: vec![stack_vma, heap_vma],
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

    fn stack_vma(&self) -> &VmArea {
        &self.vm_areas[0]
    }

    fn heap_vma(&self) -> &VmArea {
        &self.vm_areas[1]
    }

    fn heap_vma_mut(&mut self) -> &mut VmArea {
        &mut self.vm_areas[1]
    }

    pub fn add_vm_area(&mut self, start: UserVAddr, len: usize, area_type: VmAreaType) {
        self.vm_areas.push(VmArea {
            start,
            len,
            area_type,
        });
    }

    pub fn heap_end(&self) -> UserVAddr {
        self.heap_vma().end()
    }

    pub fn expand_heap_to(&mut self, new_heap_end: UserVAddr) -> Result<()> {
        let current_heap_end = self.heap_vma().end();
        if new_heap_end < current_heap_end {
            return Err(Error::new(Errno::EINVAL));
        }

        self.expand_heap_by(new_heap_end.value() - current_heap_end.value())
    }

    pub fn expand_heap_by(&mut self, increment: usize) -> Result<()> {
        let stack_bottom = self.stack_vma().start();
        let increment = align_up(increment, PAGE_SIZE);
        let heap_vma = self.heap_vma_mut();
        let new_heap_top = heap_vma
            .end()
            .add(increment)
            .map_err(|_| Error::new(Errno::ENOMEM))?;

        if new_heap_top >= stack_bottom {
            return Err(Error::new(Errno::ENOMEM));
        }

        heap_vma.len += increment;
        Ok(())
    }
}
