use core::ffi;
use std::arch::asm;

pub fn get_rbp() -> *const usize {
    let mut out;
    unsafe {
        asm!(
            "mov {out}, rbp",
            out = out(reg) out,
            options(nostack, readonly)
        );
    }
    out
}

pub fn get_rip() -> *const ffi::c_void {
    let mut out;
    unsafe {
        asm!(
            "lea {out}, [rip]",
            out = out(reg) out,
            options(nostack, readonly),
        );
    }
    out
}
