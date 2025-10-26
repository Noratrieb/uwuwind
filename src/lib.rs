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

    let frame = crate::dwarf::frame_info(arch::get_rip());

    let eh_frame = crate::dwarf::eh_frame(arch::get_rip()).unwrap();
    crate::dwarf::uwutables(eh_frame);

    stdext::abort();
}

// This is normally provided by the language runtime through the unwind info
// block. We don't want to access that usually because Rust messes with it :(.
static PERSONALITY_ROUTINE: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

pub unsafe fn set_personality_routine(routine: uw::PersonalityRoutine) {
    let ptr: *mut () = core::mem::transmute(routine);
    PERSONALITY_ROUTINE.store(ptr, core::sync::atomic::Ordering::Relaxed);
}
