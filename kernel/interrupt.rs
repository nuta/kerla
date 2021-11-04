//! Interrupt handling.

use crate::arch::{enable_irq, SpinLock};
use alloc::boxed::Box;
use core::mem::MaybeUninit;

fn empty_irq_handler() {}

const UNINITIALIZED_IRQ_HANDLER: MaybeUninit<Box<dyn FnMut() + Send + Sync>> =
    MaybeUninit::uninit();
static IRQ_HANDLERS: SpinLock<[MaybeUninit<Box<dyn FnMut() + Send + Sync>>; 256]> =
    SpinLock::new([UNINITIALIZED_IRQ_HANDLER; 256]);

pub fn init() {
    let mut handlers = IRQ_HANDLERS.lock();
    for handler in handlers.iter_mut() {
        handler.write(Box::new(empty_irq_handler));
    }
}

pub fn attach_irq<F: FnMut() + Send + Sync + 'static>(irq: u8, f: F) {
    IRQ_HANDLERS.lock()[irq as usize].write(Box::new(f));
    enable_irq(irq);
}

pub fn handle_irq(irq: u8) {
    let handler = &mut IRQ_HANDLERS.lock()[irq as usize];
    unsafe {
        (*handler.assume_init_mut())();
    }
}
