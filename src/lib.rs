#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

extern crate alloc;

#[macro_use]
extern crate tracing;

use core::{ffi, sync::atomic::AtomicPtr};

mod stdext;

pub mod uw;

mod arch;
pub mod dwarf;
mod identify;

mod walk;

#[derive(Debug, Clone, Copy)]
struct Addr(*const ());

impl Addr {
    fn voidptr(self) -> *const ffi::c_void {
        self.0.cast()
    }

    fn addr(&self) -> usize {
        self.0.addr()
    }
}

#[allow(nonstandard_style)]
pub unsafe extern "C-unwind" fn _UnwindRaiseException(
    exception_object: *mut uw::_Unwind_Exception,
) -> uw::_Unwind_Reason_Code {
    let _span = info_span!("_UnwindRaiseException", ?exception_object).entered();

    let ctx = arch::capture_context();

    let frame = crate::dwarf::frame_info(Addr(core::ptr::with_exposed_provenance(
        ctx.registers[dwarf::parse::arch::RETURN_ADDRESS],
    )));

    stdext::abort();
}
