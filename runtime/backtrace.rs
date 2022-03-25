use core::{arch::global_asm, fmt, mem::size_of, slice, str};

use alloc::boxed::Box;
use arrayvec::ArrayVec;

use crate::{address::VAddr, arch::Backtrace};

/// A symbol.
#[repr(C, packed)]
struct SymbolEntry {
    addr: u64,
    name: [u8; 56],
}

#[repr(C, packed)]
struct SymbolTable {
    magic: u32,
    num_symbols: i32,
    padding: u64,
}

extern "C" {
    static __symbol_table: SymbolTable;
}

global_asm!(
    r#"
    .rodata
    .align 8
    .global __symbol_table
    __symbol_table:
       .ascii "__SYMBOL_TABLE_START__"
       .space 2 * 1024 * 1024
       .ascii "__SYMBOL_TABLE_END__"
"#
);

struct Symbol {
    name: &'static str,
    addr: VAddr,
}

fn resolve_symbol(vaddr: VAddr) -> Option<Symbol> {
    assert!(unsafe { __symbol_table.magic } == 0xbeefbeef);

    let num_symbols = unsafe { __symbol_table.num_symbols };
    let symbols = unsafe {
        slice::from_raw_parts(
            ((&__symbol_table as *const _ as usize) + size_of::<SymbolTable>())
                as *const SymbolEntry,
            __symbol_table.num_symbols as usize,
        )
    };

    // Do a binary search.
    let mut l = -1;
    let mut r = num_symbols;
    while r - l > 1 {
        let mid = (l + r) / 2;
        if vaddr.value() >= symbols[mid as usize].addr as usize {
            l = mid;
        } else {
            r = mid;
        }
    }

    if l >= 0 {
        let symbol = &symbols[l as usize];
        Some(Symbol {
            name: unsafe { str::from_utf8_unchecked(&symbol.name) },
            addr: VAddr::new(symbol.addr as usize),
        })
    } else {
        None
    }
}

/// Prints a backtrace.
pub fn backtrace() {
    Backtrace::current_frame().traverse(|i, vaddr| {
        if let Some(symbol) = resolve_symbol(vaddr) {
            warn!(
                "    {index}: {vaddr} {symbol_name}()+0x{offset:x}",
                index = i,
                vaddr = vaddr,
                symbol_name = symbol.name,
                offset = vaddr.value() - symbol.addr.value(),
            );
        } else {
            warn!(
                "    {index}: {vaddr} (symbol unknown)",
                index = i,
                vaddr = vaddr,
            );
        }
    });
}

pub struct CapturedBacktraceFrame {
    pub vaddr: VAddr,
    pub offset: usize,
    pub symbol_name: &'static str,
}

pub struct CapturedBacktrace {
    pub trace: Box<ArrayVec<CapturedBacktraceFrame, 8>>,
}

impl CapturedBacktrace {
    /// Returns a saved backtrace.
    pub fn capture() -> CapturedBacktrace {
        let mut trace = Box::new(ArrayVec::new());
        Backtrace::current_frame().traverse(|_, vaddr| {
            if let Some(symbol) = resolve_symbol(vaddr) {
                let _ = trace.try_push(CapturedBacktraceFrame {
                    vaddr,
                    symbol_name: symbol.name,
                    offset: vaddr.value() - symbol.addr.value(),
                });
            }
        });
        CapturedBacktrace { trace }
    }
}

impl fmt::Debug for CapturedBacktrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, frame) in self.trace.iter().enumerate() {
            let _ = writeln!(
                f,
                "    #{}: {} {}()+0x{:x}",
                i + 1,
                frame.vaddr,
                frame.symbol_name,
                frame.offset
            );
        }

        Ok(())
    }
}
