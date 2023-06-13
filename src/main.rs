use uwuwind::uw;

#[repr(C)]
#[allow(dead_code)]
struct Exception {
    _uwe: uw::_Unwind_Exception,
    uwu: &'static str,
}

fn main() {
    let eh_frame = std::fs::read("eh_frame.bin").expect("could not find ./eh_frame.bin");
    unsafe {
        uwuwind::dwarf::uwutables(eh_frame.as_ptr());
    }
    return;
    /*
    unsafe {
        uwuwind::set_personality_routine(personality_routine);

        let exception = Box::into_raw(Box::new(Exception {
            _uwe: uw::_Unwind_Exception {
                exception_class: 123456,
                exception_cleanup0: |_, _| {},
                private_1: 0,
                private_2: 0,
            },
            uwu: "meow :3",
        }));

        uwuwind::_UnwindRaiseException(exception.cast::<uw::_Unwind_Exception>());
    }
    */
}

#[allow(dead_code)]
fn personality_routine(
    _version: i32,
    _actions: uw::_UnwindAction,
    _exception_class: u64,
    _exception_object: *mut uw::_Unwind_Exception,
    _context: *mut uw::_Unwind_Context,
) -> uw::_Unwind_Reason_Code {
    uw::_Unwind_Reason_Code::_URC_NO_REASON
}
