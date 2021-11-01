//! Interrupt handling.

use crate::{
    arch::{enable_irq, SpinLock},
    net::process_packets,
};
use alloc::boxed::Box;

const DEFAULT_IRQ_HANDLER: Option<Box<dyn FnMut() + Send + Sync>> = None;
static IRQ_HANDLERS: SpinLock<[Option<Box<dyn FnMut() + Send + Sync>>; 256]> =
    SpinLock::new([DEFAULT_IRQ_HANDLER; 256]);

pub fn attach_irq<F: FnMut() + Send + Sync + 'static>(irq: u8, f: F) {
    let mut handlers = IRQ_HANDLERS.lock();
    match handlers[irq as usize] {
        Some(_) => (panic!("handler for IRQ #{} is already attached", irq)),
        None => {
            handlers[irq as usize] = Some(Box::new(f));
            enable_irq(irq);
        }
    }
}

pub fn handle_irq(irq: u8) {
    if let Some(handler) = &mut IRQ_HANDLERS.lock()[irq as usize] {
        (*handler)();
        process_packets();
    }
}
