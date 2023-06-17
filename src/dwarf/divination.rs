//! # divination
//!
//! the practice of seeking knowledge of the future or the unknown by
//! supernatural means.
//!
//! we ask supernatural means (the dynamic linker) for knowledge of the future
//! (where we will find the dwarves)
//!
//! first, we ask the dynamic linker to give us the `.eh_frame` for the current
//! binary using the GNU extension (`_dl_find_object`)[https://www.gnu.org/software/libc/manual/html_node/Dynamic-Linker-Introspection.html].
//! then, we parse that as beautiful DWARF call frame information, as god (or
//! rather, the x86-64 psABI) intended.

#![allow(non_camel_case_types)]

use core::ffi;

use super::parse::Encoding;
use crate::{dwarf::parse::read_encoded, stdext::with_last_os_error_str, Addr};

#[repr(C)]
struct dl_find_object {
    dlfo_flags: ffi::c_ulonglong,
    dlfo_map_start: *const ffi::c_void,
    dlfo_map_end: *const ffi::c_void,
    dlf_link_map: *const ffi::c_void,
    /// A pointer to the `PT_GNU_EH_FRAME` segment (the `.eh_frame_hdr`
    /// section).
    dlfo_eh_frame: *const ffi::c_void,
}

extern "C" {
    fn _dl_find_object(address: *const ffi::c_void, result: *mut dl_find_object) -> ffi::c_int;
}

/// The `.eh_frame_hdr` section.
/// See <https://refspecs.linuxfoundation.org/LSB_1.3.0/gLSB/gLSB/ehframehdr.html>
/// and <https://refspecs.linuxbase.org/LSB_5.0.0/LSB-Core-generic/LSB-Core-generic/ehframechpt.html>.
#[derive(Debug)]
#[repr(C)]
struct EhFrameHeader {
    version: u8,
    eh_frame_ptr_enc: Encoding,
    fde_count_enc: Encoding,
    table_enc: Encoding,
}

fn eh_frame_hdr_ptr(addr: Addr) -> Option<*const EhFrameHeader> {
    unsafe {
        let mut out = core::mem::zeroed();
        let ret = _dl_find_object(addr.voidptr(), &mut out);
        trace!("_dl_find_object returned {ret}");
        if ret != 0 {
            with_last_os_error_str(|err| trace!("dl_find_object error: {err}"));
            return None;
        }
        if out.dlfo_eh_frame.is_null() {
            trace!("dlfo_eh_frame is null");
            return None;
        }

        let text_len = out.dlfo_map_end as usize - out.dlfo_map_start as usize;
        trace!(
            "dwarf info; map: ({:p}, {:x}), dlfo_map_end: {:p}",
            out.dlfo_map_start,
            text_len,
            out.dlfo_eh_frame
        );

        if !(out.dlfo_map_start..out.dlfo_map_end).contains(&addr.voidptr()) {
            trace!("dl_find_object returned object out of range for addr: {addr:?}");
            return None;
        }

        Some(out.dlfo_eh_frame.cast::<EhFrameHeader>())
    }
}

pub(crate) fn eh_frame(addr: Addr) -> Option<*const u8> {
    unsafe {
        let ptr = eh_frame_hdr_ptr(addr)?;
        let header = ptr.read();

        // ptr now points to `eh_frame_ptr`.
        let ptr = ptr.cast::<u8>().add(4);

        if header.version != 1 {
            trace!("eh_frame_hdr version is not 1");
            return None;
        }

        trace!("eh_frame_hdr: {:#?}", header);

        let (read_size, eh_frame_ptr) = read_encoded(ptr, header.eh_frame_ptr_enc);
        let ptr = ptr.add(read_size);
        let (_read_size, fde_count) = read_encoded(ptr, header.fde_count_enc);

        trace!("eh_frame: {eh_frame_ptr:?}");
        trace!("fde_count: {fde_count:?}");

        trace!(
            "eh_frame start: {:x?}",
            core::slice::from_raw_parts(eh_frame_ptr as *const u8, 15)
        );

        crate::dwarf::uwutables(eh_frame_ptr as *const u8);

        Some(eh_frame_ptr as *const u8)
    }
}
