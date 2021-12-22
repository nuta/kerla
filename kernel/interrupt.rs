//! Interrupt handling.

use alloc::{boxed::Box, vec::Vec};
use kerla_runtime::{arch::enable_irq, spinlock::SpinLock};

use crate::{deferred_job::run_deferred_jobs, interval_work};

type IrqHandler = dyn FnMut() + Send + Sync;
const NUM_IRQ_NUMBERS: usize = 256;

/// Holds the interrupt handlers. The index is the IRQ number and this vector
/// contains `NUM_IRQ_NUMBERS` entries.
static IRQ_VECTORS: SpinLock<Vec<IrqVector>> = SpinLock::new(Vec::new());

pub struct IrqVector {
    handlers: Vec<Box<IrqHandler>>,
}

impl IrqVector {
    pub fn new() -> IrqVector {
        IrqVector {
            handlers: Vec::new(),
        }
    }

    pub fn handlers_mut(&mut self) -> &mut [Box<IrqHandler>] {
        &mut self.handlers
    }

    pub fn add_handler(&mut self, f: Box<IrqHandler>) {
        self.handlers.push(f);
    }
}

pub fn attach_irq(irq: u8, f: Box<IrqHandler>) {
    debug_assert!((irq as usize) < NUM_IRQ_NUMBERS);
    IRQ_VECTORS.lock()[irq as usize].add_handler(f);
    enable_irq(irq);
}

pub fn handle_irq(irq: u8) {
    {
        debug_assert!((irq as usize) < NUM_IRQ_NUMBERS);
        let mut vectors = IRQ_VECTORS.lock();
        for handler in vectors[irq as usize].handlers_mut() {
            handler();
        }

        // `vectors` is dropped here to release IRQ_HANDLERS's lock since
        // we re-enable interrupts just before running deferred jobs.
    }

    // TODO: Re-enable interrupts to make deferred jobs preemptive.

    // So-called "bottom half" in Linux kernel. Execute time-consuming but
    // non-critical work like processing packets.
    interval_work();
    run_deferred_jobs();
}

pub fn init() {
    let mut vectors = IRQ_VECTORS.lock();
    for _ in 0..NUM_IRQ_NUMBERS {
        vectors.push(IrqVector::new());
    }
}
