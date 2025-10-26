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
    rest: (),
}

#[instrument]
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

#[instrument]
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

        trace!("eh_frame_hdr: {:?}", header);

        let (read_size, eh_frame_ptr) = read_encoded(ptr, header.eh_frame_ptr_enc, None);
        let ptr = ptr.add(read_size);
        let (_read_size, fde_count) = read_encoded(ptr, header.fde_count_enc, None);

        trace!("eh_frame: {eh_frame_ptr:?}");
        trace!("fde_count: {fde_count:?}");

        trace!(
            "eh_frame start: {:x?}",
            core::slice::from_raw_parts(eh_frame_ptr as *const u8, 15)
        );

        Some(eh_frame_ptr as *const u8)
    }
}

#[instrument]
pub(crate) fn frame_info(addr: Addr) -> Option<()> {
    unsafe {
        let header_ptr = eh_frame_hdr_ptr(addr)?;
        let eh_frame_header_addr = header_ptr.addr();
        let header = header_ptr.read();

        let ptr = (&raw const (*header_ptr).rest).cast::<u8>();
        let (eh_frame_ptr_size, eh_frame_ptr) = read_encoded(ptr, header.eh_frame_ptr_enc, None);
        let ptr = ptr.add(eh_frame_ptr_size);

        let (fde_count_size, fde_count) =
            read_encoded(ptr, header.fde_count_enc, Some(eh_frame_ptr));

        trace!(?header.table_enc);

        let table_ptr = ptr.add(fde_count_size);

        let mut walk_table_ptr = table_ptr;
        for i in 0..fde_count {
            let (read, initial_loc) =
                read_encoded(walk_table_ptr, header.table_enc, Some(eh_frame_header_addr));
            walk_table_ptr = walk_table_ptr.add(read);
            let (read, address) =
                read_encoded(walk_table_ptr, header.table_enc, Some(eh_frame_header_addr));
            walk_table_ptr = walk_table_ptr.add(read);

            trace!(idx = ?i, "eh_frame_hdr table initial_loc={initial_loc:x} address={address:x}");
        }

        let table_half_entry_size = header.table_enc.size();

        let mut base = 0;
        let mut len = fde_count;
        let found_fde;
        loop {
            if len == 1 {
                found_fde = Some(base);
                break;
            }

            let mid = base + len / 2;
            let mid_ptr = table_ptr.byte_add(mid * table_half_entry_size * 2);

            let (_, value) = read_encoded(mid_ptr, header.table_enc, Some(eh_frame_header_addr));

            debug!(
                ?base,
                ?len,
                ?mid,
                "binary searching for {addr:?}: {value:x}"
            );

            match addr.addr().cmp(&value) {
                core::cmp::Ordering::Less => {
                    len = mid - base;
                }
                core::cmp::Ordering::Equal => {
                    found_fde = Some(mid);
                    break;
                }
                core::cmp::Ordering::Greater => {
                    len = len - (mid - base);
                    base = mid;
                }
            }
        }

        debug!("found FDE idx in binary search {found_fde:?}");

        let fde_table_ptr = table_ptr.byte_add(found_fde.unwrap() * table_half_entry_size * 2);
        let (_, fde_address) = read_encoded(
            fde_table_ptr.byte_add(table_half_entry_size),
            header.table_enc,
            Some(eh_frame_header_addr),
        );

        trace!("found FDE at address {fde_address:x}");

        let fde_ptr = core::ptr::with_exposed_provenance::<u8>(fde_address);

        fde_ptr.read_volatile();

        trace!("ptr is valid");

        trace!("FDE offset to .eh_frame: {:x}", fde_ptr.addr() - (eh_frame_ptr));

        let fde = crate::dwarf::parse::parse_fde_from_ptr(fde_ptr, eh_frame_ptr).unwrap();

        todo!()
    }
}
