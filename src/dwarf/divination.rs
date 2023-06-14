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

use core::{ffi, fmt};

use crate::stdext::with_last_os_error_str;

use crate::Addr;

#[repr(C)]
struct dl_find_object {
    dlfo_flags: ffi::c_ulonglong,
    dlfo_map_start: *const ffi::c_void,
    dlfo_map_end: *const ffi::c_void,
    dlf_link_map: *const ffi::c_void,
    /// A pointer to the `PT_GNU_EH_FRAME` segment (the `.eh_frame_hdr` section).
    dlfo_eh_frame: *const ffi::c_void,
}

extern "C" {
    fn _dl_find_object(address: *const ffi::c_void, result: *mut dl_find_object) -> ffi::c_int;
}

/// The `.eh_frame_hdr` section.
/// See <https://refspecs.linuxfoundation.org/LSB_1.3.0/gLSB/gLSB/ehframehdr.html>
/// and <https://refspecs.linuxbase.org/LSB_5.0.0/LSB-Core-generic/LSB-Core-generic/ehframechpt.html>.
#[derive(Debug)]
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
        let ptr = ptr.cast::<u8>().add(4);

        if header.version != 1 {
            trace!("eh_frame_hdr version is not 1");
            return None;
        }

        trace!("eh_frame_hdr: {:#?}", header);

        let (ptr, eh_frame_ptr) = read_encoded(ptr, header.eh_frame_ptr_enc);
        let (_ptr, fde_count) = read_encoded(ptr, header.fde_count_enc);

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

unsafe fn read_encoded(ptr: *const u8, encoding: Encoding) -> (*const u8, usize) {
    let (new_ptr, value) = match encoding.format() {
        ValueFormat::DW_EH_PE_uleb128 => todo!("uleb128"),
        ValueFormat::DW_EH_PE_udata2 => (ptr.add(2), ptr.cast::<u16>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_udata4 => (ptr.add(4), ptr.cast::<u32>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_udata8 => (ptr.add(8), ptr.cast::<u64>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_sleb128 => todo!("sleb128"),
        ValueFormat::DW_EH_PE_sdata2 => (ptr.add(2), ptr.cast::<i16>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_sdata4 => (ptr.add(4), ptr.cast::<i32>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_sdata8 => (ptr.add(8), ptr.cast::<i64>().read_unaligned() as usize),
    };

    let value = match encoding.application() {
        ValueApplication::DW_EH_PE_absptr => value,
        ValueApplication::DW_EH_PE_pcrel => ((value as isize) + (ptr as isize)) as usize,
        ValueApplication::DW_EH_PE_textrel => todo!("textrel"),
        ValueApplication::DW_EH_PE_datarel => todo!("datarel"),
        ValueApplication::DW_EH_PE_funcrel => todo!("funcrel"),
        ValueApplication::DW_EH_PE_aligned => todo!("aligned"),
    };

    (new_ptr, value)
}

struct Encoding(u8);
impl Encoding {
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

impl fmt::Debug for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} | {:?}", self.application(), self.format())
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
