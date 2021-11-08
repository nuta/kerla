//! Interrupt handling.

use crate::arch::{enable_irq, SpinLock};
use alloc::boxed::Box;
use core::mem::MaybeUninit;
use kerla_utils::bitmap::BitMap;

fn empty_irq_handler() {}

type IrqHandler = dyn FnMut() + Send + Sync;
const UNINITIALIZED_IRQ_HANDLER: MaybeUninit<Box<IrqHandler>> = MaybeUninit::uninit();
static IRQ_HANDLERS: SpinLock<[MaybeUninit<Box<IrqHandler>>; 256]> =
    SpinLock::new([UNINITIALIZED_IRQ_HANDLER; 256]);
static ATTACHED_IRQS: SpinLock<BitMap<32 /* = 256 / 8 */>> = SpinLock::new(BitMap::zeroed());

pub fn init() {
    let mut handlers = IRQ_HANDLERS.lock();
    for handler in handlers.iter_mut() {
        handler.write(Box::new(empty_irq_handler));
    }
}

pub fn attach_irq<F: FnMut() + Send + Sync + 'static>(irq: u8, f: F) {
    let mut attached_irq_map = ATTACHED_IRQS.lock();
    match attached_irq_map.get(irq as usize) {
        Some(true) => panic!("handler for IRQ #{} is already attached", irq),
        Some(false) => {
            attached_irq_map.set(irq as usize);
            IRQ_HANDLERS.lock()[irq as usize].write(Box::new(f));
            enable_irq(irq);
        }
        None => panic!("IRQ #{} is out of bound", irq),
    }
}

pub fn handle_irq(irq: u8) {
    let handler = &mut IRQ_HANDLERS.lock()[irq as usize];
    unsafe {
        (*handler.assume_init_mut())();
    }
}
