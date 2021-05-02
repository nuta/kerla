use crate::{arch::UserVAddr, ctypes::c_int, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Signal {
    SIGCHLD = 17,
}

const SIGMAX: usize = 32;

pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

#[derive(Clone, Copy)]
pub enum SigAction {
    Ignore,
    Handler { handler: UserVAddr },
}

pub struct SignalDelivery {
    pending: u32,
    actions: [SigAction; SIGMAX],
}

impl SignalDelivery {
    pub fn new() -> SignalDelivery {
        SignalDelivery {
            pending: 0,
            actions: [SigAction::Ignore; SIGMAX],
        }
    }

    pub fn set_action(&mut self, signum: c_int, action: SigAction) -> Result<()> {
        if signum as usize > SIGMAX {
            return Err(Errno::EINVAL.into());
        }

        self.actions[signum as usize] = action;
        Ok(())
    }

    pub fn pop_pending(&mut self) -> Option<(Signal, SigAction)> {
        if self.pending == 0 {
            return None;
        }

        let signal_no = self.pending.trailing_zeros();
        self.pending &= !(1 << signal_no);
        let signal = match signal_no {
            _ if signal_no == Signal::SIGCHLD as u32 => Signal::SIGCHLD,
            _ => unreachable!(),
        };

        Some((signal, self.actions[signal_no as usize]))
    }

    pub fn signal(&mut self, signal: Signal) {
        self.pending |= 1 << (signal as u32);
    }
}
