//! Interrupt handling.

use alloc::boxed::Box;
use core::mem::MaybeUninit;
use kerla_runtime::{arch::enable_irq, spinlock::SpinLock};
use kerla_utils::bitmap::BitMap;

use crate::{deferred_job::run_deferred_jobs, interval_work};

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

pub fn attach_irq(irq: u8, f: Box<dyn FnMut() + Send + Sync + 'static>) {
    let mut attached_irq_map = ATTACHED_IRQS.lock();
    match attached_irq_map.get(irq as usize) {
        Some(true) => panic!("handler for IRQ #{} is already attached", irq),
        Some(false) => {
            attached_irq_map.set(irq as usize);
            IRQ_HANDLERS.lock()[irq as usize].write(f);
            enable_irq(irq);
        }
        None => panic!("IRQ #{} is out of bound", irq),
    }
}

pub fn handle_irq(irq: u8) {
    {
        let handler = &mut IRQ_HANDLERS.lock()[irq as usize];
        unsafe {
            (*handler.assume_init_mut())();
        }

        // `handler` is dropped here to release IRQ_HANDLERS's lock since
        // we re-enable interrupts just before running deferred jobs.
    }

    // TODO: Re-enable interrupts to make deferred jobs preemptive.

    // So-called "bottom half" in Linux kernel. Execute time-consuming but
    // non-critical work like processing packets.
    interval_work();
    run_deferred_jobs();
}
