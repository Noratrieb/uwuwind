//! # divination
//!
//! the practice of seeking knowledge of the future or the unknown by supernatural means.
//!
//! we ask supernatural means (the dynamic linker) for knowledge of the future
//! (where we will find the dwarves)
//!
//! first, we ask the dynamic linker to give us the `.eh_frame` for the current binary using
//! the GNU extension (`_dl_find_object`)[https://www.gnu.org/software/libc/manual/html_node/Dynamic-Linker-Introspection.html].
//! then, we parse that as beautiful DWARF call frame information, as god (or rather, the x86-64 psABI) intended.

#![allow(non_camel_case_types)]

use core::{ffi, fmt, ptr::addr_of};

use crate::stdext::with_last_os_error_str;

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

#[derive(Debug, Clone, Copy)]
pub struct DwarfInfo {
    /// A pointer to the `PT_GNU_EH_FRAME` segment (the `.eh_frame_hdr` section).
    pub(crate) eh_frame_hdr: *const u8,
}

/// The `.eh_frame_hdr` section.
/// See <https://refspecs.linuxfoundation.org/LSB_1.3.0/gLSB/gLSB/ehframehdr.html>
/// and <https://refspecs.linuxbase.org/LSB_5.0.0/LSB-Core-generic/LSB-Core-generic/ehframechpt.html>.
struct EhFrameHeader {
    version: u8,
    eh_frame_ptr_enc: EhHeaderEncoded,
    fde_count_enc: EhHeaderEncoded,
    table_enc: EhHeaderEncoded,
    encoded_fields: (),
}

impl EhFrameHeader {
    unsafe fn encoded_fields(&self) -> *const u8 {
        addr_of!((*self).encoded_fields).cast::<u8>()
    }

    unsafe fn eh_frame(&self) -> Option<*const u8> {
        let ValueFormat::DW_EH_PE_sdata4 = self.eh_frame_ptr_enc.format() else {
            return None;
        };
        let ValueApplication::DW_EH_PE_pcrel = self.eh_frame_ptr_enc.application() else {
            return None;
        };

        let eh_frame_ptr = unsafe { self.encoded_fields().cast::<i32>().read_unaligned() };

        Some(
            self.encoded_fields()
                .cast::<u8>()
                .offset(eh_frame_ptr as isize),
        )
    }

    fn fde_count(&self) -> Option<u64> {
        let ValueFormat::DW_EH_PE_udata4 = self.fde_count_enc.format() else {
            return None;
        };
        let ValueApplication::DW_EH_PE_absptr = self.fde_count_enc.application() else {
            return None;
        };
        let fde_count = unsafe { self.encoded_fields().add(4).cast::<u32>().read() };
        Some(fde_count as _)
    }
}

impl fmt::Debug for EhFrameHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EhFrameHeader")
            .field("version", &self.version)
            .field("eh_frame_ptr_enc", &self.eh_frame_ptr_enc)
            .field("fde_count_enc", &self.fde_count_enc)
            .field("table_enc", &self.table_enc)
            .field("fde_count", &self.fde_count())
            .finish()
    }
}

struct EhHeaderEncoded(u8);
impl EhHeaderEncoded {
    fn format(&self) -> ValueFormat {
        match self.0 & 0b1111 {
            0x01 => ValueFormat::DW_EH_PE_uleb128,
            0x02 => ValueFormat::DW_EH_PE_udata2,
            0x03 => ValueFormat::DW_EH_PE_udata4,
            0x04 => ValueFormat::DW_EH_PE_udata8,
            0x09 => ValueFormat::DW_EH_PE_sleb128,
            0x0A => ValueFormat::DW_EH_PE_sdata2,
            0x0B => ValueFormat::DW_EH_PE_sdata4,
            0x0C => ValueFormat::DW_EH_PE_sdata8,
            _ => panic!("Invalid header value format"),
        }
    }
    fn application(&self) -> ValueApplication {
        match self.0 >> 4 {
            0x0 => ValueApplication::DW_EH_PE_absptr,
            0x1 => ValueApplication::DW_EH_PE_pcrel,
            0x2 => ValueApplication::DW_EH_PE_textrel,
            0x3 => ValueApplication::DW_EH_PE_datarel,
            0x4 => ValueApplication::DW_EH_PE_funcrel,
            0x5 => ValueApplication::DW_EH_PE_aligned,
            v => panic!("invalid header value application: {v}"),
        }
    }
}

impl fmt::Debug for EhHeaderEncoded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} | {:?}", self.application(), self.format())
    }
}

pub fn dwarf_info(addr: *const ffi::c_void) -> Option<DwarfInfo> {
    unsafe {
        let mut out = core::mem::zeroed();
        let ret = _dl_find_object(addr, &mut out);
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

        if !(out.dlfo_map_start..out.dlfo_map_end).contains(&addr) {
            trace!("dl_find_object returned object out of range for addr: {addr:p}");
            return None;
        }

        let header = &*out.dlfo_eh_frame.cast::<EhFrameHeader>();

        if header.version != 1 {
            trace!("eh_frame_hdr version is not 1");
            return None;
        }
        trace!("eh_frame_hdr: {:#?}", header);

        let Some(ptr) = header.eh_frame() else {
            trace!("could not find .eh_frame");
            return None;
        };
        trace!("eh_frame pointer: {ptr:?}");

        trace!("eh_frame start: {:?}", core::slice::from_raw_parts(ptr, 10));

        crate::stdext::abort();
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum ValueFormat {
    /// Unsigned value is encoded using the Little Endian Base 128 (LEB128) as defined by DWARF Debugging Information Format, Revision 2.0.0 (July 27, 1993).
    DW_EH_PE_uleb128 = 0x01,
    /// A 2 bytes unsigned value.
    DW_EH_PE_udata2 = 0x02,
    /// A 4 bytes unsigned value.
    DW_EH_PE_udata4 = 0x03,
    /// An 8 bytes unsigned value.
    DW_EH_PE_udata8 = 0x04,
    /// Signed value is encoded using the Little Endian Base 128 (LEB128) as defined by DWARF Debugging Information Format, Revision 2.0.0 (July 27, 1993).
    DW_EH_PE_sleb128 = 0x09,
    /// A 2 bytes signed value.
    DW_EH_PE_sdata2 = 0x0A,
    /// A 4 bytes signed value.
    DW_EH_PE_sdata4 = 0x0B,
    /// An 8 bytes signed value.
    DW_EH_PE_sdata8 = 0x0C,
}

#[derive(Debug)]
#[repr(u8)]
enum ValueApplication {
    DW_EH_PE_absptr = 0x00,
    ///	Value is relative to the current program counter.
    DW_EH_PE_pcrel = 0x10,
    ///	Value is relative to the beginning of the .text section.
    DW_EH_PE_textrel = 0x20,
    ///	Value is relative to the beginning of the .got or .eh_frame_hdr section.
    DW_EH_PE_datarel = 0x30,
    ///	Value is relative to the beginning of the function.
    DW_EH_PE_funcrel = 0x40,
    ///	Value is aligned to an address unit sized boundary.
    DW_EH_PE_aligned = 0x50,
}
