#![no_std]
#![allow(dead_code)]

use core::sync::atomic::AtomicPtr;

// Get the macros into our local prelude.
#[macro_use]
mod stdext;

pub mod uw;

mod arch;
mod dwarf;
mod identify;

mod walk;

#[allow(nonstandard_style)]
pub unsafe extern "C" fn _UnwindRaiseException(
    exception_object: *mut uw::_Unwind_Exception,
) -> uw::_Unwind_Reason_Code {
    trace!("someone raised an exception with addr {exception_object:p}");
    crate::dwarf::dwarf_info(arch::get_rip() as _);

    // walk::fp::walk();

    stdext::abort();
}

// This is normally provided by the language runtime through the unwind info block.
// We don't want to access that usually because Rust messes with it :(.
static PERSONALITY_ROUTINE: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

pub unsafe fn set_personality_routine(routine: uw::PersonalityRoutine) {
    let ptr: *mut () = core::mem::transmute(routine);
    PERSONALITY_ROUTINE.store(ptr, core::sync::atomic::Ordering::Relaxed);
}
