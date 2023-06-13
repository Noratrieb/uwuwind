use core::arch::asm;

use crate::Addr;

pub(crate) fn get_rbp() -> Addr {
    let mut out;
    unsafe {
        asm!(
            "mov {out}, rbp",
            out = out(reg) out,
            options(nostack, readonly)
        );
    }
    Addr(out)
}

pub(crate) fn get_rip() -> Addr {
    let mut out;
    unsafe {
        asm!(
            "lea {out}, [rip]",
            out = out(reg) out,
            options(nostack, readonly),
        );
    }
    Addr(out)
}
