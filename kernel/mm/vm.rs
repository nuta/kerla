use crate::fs::inode::FileLike;
use crate::{
    arch::{USER_STACK_TOP, USER_VALLOC_BASE, USER_VALLOC_END},
    result::{Errno, Result},
};
use alloc::sync::Arc;
use alloc::vec::Vec;
use kerla_runtime::{
    address::UserVAddr,
    arch::{PageTable, PAGE_SIZE},
};
use kerla_utils::alignment::{align_up, is_aligned};

#[derive(Clone)]
pub enum VmAreaType {
    Anonymous,
    File {
        file: Arc<dyn FileLike>,
        offset: usize,
        file_size: usize,
    },
}

#[derive(Clone)]
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
        self.start.add(self.len)
    }

    pub fn offset_in_vma(&self, vaddr: UserVAddr) -> usize {
        debug_assert!(self.contains(vaddr));
        vaddr.value() - self.start.value()
    }

    pub fn contains(&self, vaddr: UserVAddr) -> bool {
        self.start.value() <= vaddr.value() && vaddr.value() < self.start.value() + self.len
    }

    pub fn overlaps(&self, other: UserVAddr, len: usize) -> bool {
        self.start.value() <= other.value() + len && other.value() < self.start.value() + self.len
    }
}

pub struct Vm {
    page_table: PageTable,
    vm_areas: Vec<VmArea>,
    valloc_next: UserVAddr,
}

impl Vm {
    pub fn new(stack_bottom: UserVAddr, heap_bottom: UserVAddr) -> Result<Vm> {
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

        Ok(Vm {
            page_table: PageTable::new()?,
            // The order of elements must be unchanged because `stack_vma_mut()`
            // and `heap_vma_mut` depends on it.
            vm_areas: vec![stack_vma, heap_vma],
            valloc_next: USER_VALLOC_BASE,
        })
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

    pub fn add_vm_area(
        &mut self,
        start: UserVAddr,
        len: usize,
        area_type: VmAreaType,
    ) -> Result<()> {
        start.access_ok(len)?;

        if !self.is_free_vaddr_range(start, len) {
            return Err(Errno::EINVAL.into());
        }

        self.vm_areas.push(VmArea {
            start,
            len,
            area_type,
        });

        Ok(())
    }

    pub fn heap_end(&self) -> UserVAddr {
        self.heap_vma().end()
    }

    pub fn expand_heap_to(&mut self, new_heap_end: UserVAddr) -> Result<()> {
        let current_heap_end = self.heap_vma().end();
        if new_heap_end < current_heap_end {
            return Err(Errno::EINVAL.into());
        }

        self.expand_heap_by(new_heap_end.value() - current_heap_end.value())
    }

    pub fn expand_heap_by(&mut self, increment: usize) -> Result<()> {
        let stack_bottom = self.stack_vma().start();
        let increment = align_up(increment, PAGE_SIZE);
        let heap_vma = self.heap_vma_mut();
        let new_heap_top = heap_vma.end().add(increment);

        if new_heap_top >= stack_bottom {
            return Err(Errno::ENOMEM.into());
        }

        heap_vma.len += increment;
        Ok(())
    }

    pub fn fork(&self) -> Result<Vm> {
        Ok(Vm {
            page_table: PageTable::duplicate_from(&self.page_table)?,
            vm_areas: self.vm_areas.clone(),
            valloc_next: self.valloc_next,
        })
    }

    pub fn is_free_vaddr_range(&self, start: UserVAddr, len: usize) -> bool {
        self.vm_areas.iter().all(|area| !area.overlaps(start, len))
    }

    pub fn alloc_vaddr_range(&mut self, len: usize) -> Result<UserVAddr> {
        let next = self.valloc_next;
        self.valloc_next = self.valloc_next.add(align_up(len, PAGE_SIZE));
        if self.valloc_next >= USER_VALLOC_END {
            return Err(Errno::ENOMEM.into());
        }

        Ok(next)
    }
}
