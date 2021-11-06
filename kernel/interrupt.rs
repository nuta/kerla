//! Interrupt handling.

use crate::arch::{enable_irq, SpinLock};
use alloc::boxed::Box;
use core::mem::MaybeUninit;

fn empty_irq_callback() {}

struct IrqHandler {
    callback: Box<dyn FnMut() + Send + Sync>,
    attached: bool,
}

const UNINITIALIZED_IRQ_HANDLER: MaybeUninit<IrqHandler> = MaybeUninit::uninit();
static IRQ_HANDLERS: SpinLock<[MaybeUninit<IrqHandler>; 256]> =
    SpinLock::new([UNINITIALIZED_IRQ_HANDLER; 256]);

pub fn init() {
    let mut handlers = IRQ_HANDLERS.lock();
    for handler in handlers.iter_mut() {
        handler.write(IrqHandler {
            callback: Box::new(empty_irq_callback),
            attached: false,
        });
    }
}

pub fn attach_irq<F: FnMut() + Send + Sync + 'static>(irq: u8, callback: F) {
    let h = &mut IRQ_HANDLERS.lock()[irq as usize];
    let irq_handler = unsafe { h.assume_init_mut() };
    if irq_handler.attached {
        panic!("handler for IRQ #{} is already attached", irq);
    } else {
        irq_handler.attached = true;
        irq_handler.callback = Box::new(callback);
        enable_irq(irq);
    }
}

pub fn handle_irq(irq: u8) {
    let handler = &mut IRQ_HANDLERS.lock()[irq as usize];
    unsafe {
        (*handler.assume_init_mut().callback)();
    }
}
