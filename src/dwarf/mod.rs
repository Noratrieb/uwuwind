//! this implements the stuff necessary to get the uwutables for actual unwinding
//!
//! for this we need a DWARF parser and a DWARF call frame information interpreter (yes, that shit is basically a programming
//! language). See https://dwarfstd.org/doc/DWARF5.pdf for more information if more information is desired.

mod divination;
mod parse;

pub use divination::{dwarf_info, DwarfInfo};

pub fn uwutables(dwarf_info: DwarfInfo) {
    trace!("getting uwutables from {:p}", dwarf_info.dwarf);
}
