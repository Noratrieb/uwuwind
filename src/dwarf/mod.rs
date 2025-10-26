//! this implements the stuff necessary to get the uwutables for actual
//! unwinding
//!
//! for this we need a DWARF parser and a DWARF call frame information
//! interpreter (yes, that shit is basically a programming language).
//! See https://dwarfstd.org/doc/DWARF5.pdf §6.4 for more information if more information is desired.
//! Note that https://refspecs.linuxbase.org/LSB_5.0.0/LSB-Core-generic/LSB-Core-generic/ehframechpt.html
//! contains more details on the precise format, which is slightly different
//! from .debug_frame from DWARF.

mod divination;
mod parse;

pub(crate) use divination::{eh_frame, frame_info};

/// The `.eh_frame` section contains a list of call frame information records.
/// Each CFI contains a CIE followed be one or more FDE records.
#[instrument]
pub unsafe fn uwutables(eh_frame: *const u8) {
    trace!("getting uwutables from {:p}", eh_frame);
    unsafe {
        parse::parse_cfi(eh_frame);
    }
}
