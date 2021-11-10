//! A thread-safe and almost readonly cell.
//!
//! Currently it is just a wrapper of AtomicCell, but we may optimize it to
//! eliminate atomic operations in reading.
use core::ops::Deref;

use crossbeam::atomic::AtomicCell;

pub struct StaticCell<T: Copy> {
    value: AtomicCell<T>,
}

impl<T: Copy> StaticCell<T> {
    pub const fn new(value: T) -> StaticCell<T> {
        StaticCell {
            value: AtomicCell::new(value),
        }
    }

    pub fn store(&self, value: T) {
        self.value.store(value)
    }

    pub fn load(&self) -> T {
        // TODO: Stop using AtomicCell: avoid using atomic operations.
        self.value.load()
    }
}

// TODO: Implement Deref for StaticCell<&'static T>
