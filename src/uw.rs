#![allow(nonstandard_style)] // Closely follow the spec here

#[repr(C)]
pub enum _Unwind_Reason_Code {
    _URC_NO_REASON = 0,
    /// This indicates that a different runtime caught this exception.
    /// Nested foreign exceptions, or re-throwing a foreign exception, result in
    /// undefined behavior.
    _URC_FOREIGN_EXCEPTION_CAUGHT = 1,
    /// The personality routine encountered an error during phase 1, other than the specific error codes defined.
    _URC_FATAL_PHASE1_ERROR = 3,
    /// The personality routine encountered an error during phase 2, for instance a stack corruption.
    _URC_FATAL_PHASE2_ERROR = 2,
    _URC_NORMAL_STOP = 4,
    /// The unwinder encountered the end of the stack during phase 1, without finding a handler.
    /// The unwind runtime will not have modified the stack.
    /// The C++ runtime will normally call uncaught_exception() in this case
    _URC_END_OF_STACK = 5,
    _URC_HANDLER_FOUND = 6,
    _URC_INSTALL_CONTEXT = 7,
    _URC_CONTINUE_UNWIND = 8,
}

#[repr(C, align(8))]
pub struct _Unwind_Exception {
    pub exception_class: u64,
    pub exception_cleanup0: _Unwind_Exception_Cleanup_Fn,
    pub private_1: u64,
    pub private_2: u64,
}

pub type _Unwind_Exception_Cleanup_Fn =
    fn(reason: _Unwind_Reason_Code, exc: *const _Unwind_Exception);

/// The _Unwind_Context type is an opaque type used to refer to a system-specific data structure used by the system unwinder.
/// This context is created and destroyed by the system, and passed to the personality routine during unwinding
pub struct _Unwind_Context {}

pub type PersonalityRoutine = fn(
    version: i32,
    actions: _UnwindAction,
    exceptionClass: u64,
    exception_object: *mut _Unwind_Exception,
    context: *mut _Unwind_Context,
) -> _Unwind_Reason_Code;

pub type _UnwindAction = i32;

const _UA_SEARCH_PHASE: _UnwindAction = 1;
const _UA_CLEANUP_PHASE: _UnwindAction = 2;
const _UA_HANDLER_FRAME: _UnwindAction = 4;
const _UA_FORCE_UNWIND: _UnwindAction = 8;
