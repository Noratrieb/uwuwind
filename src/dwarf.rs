use core::ffi;
use std::{ffi::CStr, fmt::Debug};

use gimli::UnwindTable;

#[allow(non_camel_case_types)]
#[repr(C)]
struct dl_find_object {
    dlfo_flags: ffi::c_ulonglong,
    dlfo_map_start: *const ffi::c_void,
    dlfo_map_end: *const ffi::c_void,
    dlf_link_map: *const ffi::c_void,
    dlfo_eh_frame: *const ffi::c_void,
}

extern "C" {
    fn _dl_find_object(address: *const ffi::c_void, result: *mut dl_find_object) -> ffi::c_int;
}

#[derive(Debug)]
pub struct DwarfInfo {
    /// The text segment
    map: *const [u8],
    /// PT_GNU_EH_FRAME
    dwarf: *const u8,
}

pub fn dwarf_info(addr: *const ffi::c_void) -> Option<DwarfInfo> {
    unsafe {
        let mut out = core::mem::zeroed();
        let ret = _dl_find_object(addr, &mut out);
        trace!("dl_find_object returned {ret}");
        if ret != 0 {
            return None;
        }
        if out.dlfo_eh_frame.is_null() {
            return None;
        }

        let text_len = out.dlfo_map_end as usize - out.dlfo_map_start as usize;

        trace!(
            "dwarf info; map: ({:p}, {:x}), dwarf: {:p}",
            out.dlfo_map_start,
            text_len,
            out.dlfo_eh_frame
        );
        Some(DwarfInfo {
            map: core::ptr::slice_from_raw_parts(out.dlfo_map_start as _, text_len),
            dwarf: out.dlfo_eh_frame as _,
        })
    }
}

pub fn uwutables() {
    //let UnwindTable;
}