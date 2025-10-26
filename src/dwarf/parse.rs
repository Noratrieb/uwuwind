//! Implements parsing and processing of DWARF call frame information.
//!
//! Source: https://dwarfstd.org/doc/DWARF5.pdf §6.4 Call Frame Information
//!
//! The CFI is a very large table of the following structure:
//! ```text
//! LOC CFA R0 R1 ... RN
//! L0
//! L1
//! ...
//! LN
//! ```
//!
//! The first column is the address for every location that contains code in a
//! program (a relative offset in shared object files). The remaining columns
//! contain unwinding rules that are associated with the indicated location.
//!
//! The CFA column defines the rule which computes the Canonical Frame Address
//! value; it may be either a register and a signed offset that are added
//! together, or a DWARF expression that is evaluated.
//!
//! The remaining columns describe register numbers that indicate whether a
//! register has been saved and the rule to find the value for the previous
//! frame.
#![allow(non_upper_case_globals)]

#[cfg(test)]
mod tests;

use alloc::{format, string::String};
use core::{ffi::CStr, fmt};

/// The dwarf is invalid. This is fatal and should never happen.
#[derive(Debug)]
pub struct Error(String);

type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug)]
pub struct Expr;

#[derive(Debug)]
enum RegisterRule {
    /// A register that has this rule has no recoverable value in the previous
    /// frame. (By convention, it is not preserved by a callee.)
    Undefined,
    /// This register has not been modified from the previous frame.
    /// (By convention, it is preserved by the callee, but the callee has not
    /// modified it.)
    SameValue,
    /// The previous value of this register is saved at the address CFA+N where
    /// CFA is the current CFA value and N is a signed offset
    Offset(isize),
    /// The previous value of this register is the value CFA+N where CFA is the
    /// current CFA value and N is a signed offset.
    ValOffset(isize),
    /// The previous value of this register is stored in another register
    /// numbered R.
    Register(u16),
    /// The previous value of this register is located at the address produced
    /// by executing the DWARF expression E (see Section 2.5 on page 26)
    Expression(Expr),
    /// The previous value of this register is the value produced by executing
    /// the DWARF expression E (see Section 2.5 on page 26).
    ValExpression(Expr),
    ///  The rule is defined externally to this specification by the augmenter.
    Architectural,
}

type Id = u32;

#[derive(Debug)]
pub struct ULeb128(u128);
impl ULeb128 {
    fn parse() -> Self {
        todo!()
    }
}
#[derive(Debug)]
pub struct ILeb128(i128);
impl ILeb128 {
    fn parse() -> Self {
        todo!()
    }
}

/// Common Information Entry
#[derive(Debug, PartialEq)]
pub struct Cie<'a> {
    /// A null-terminated UTF-8 string that identifies the augmentation to this
    /// CIE or to the FDEs that use it. If a reader encounters an
    /// augmentation string that is unexpected, then only the following
    /// fields can be read:
    /// - CIE: length, CIE_id, version, augmentation
    /// - FDE: length, CIE_pointer, initial_location, address_range
    ///
    /// If there is no augmentation, this value is a zero byte.
    ///
    /// The augmentation string allows users to indicate that there is
    /// additional target-specific information in the CIE or FDE which is
    /// needed to virtually unwind a stack frame. For example, this might be
    /// information about dynamically allocated data which needs to be freed
    /// on exit from the routine.
    ///
    /// Because the .debug_frame section is useful independently of any
    /// .debug_info section, the augmentation string always uses UTF-8
    /// encoding.
    pub augmentation: Option<AugmentationData>,
    pub augmentation_string: &'a str,
    /// A constant that is factored out of all advance location instructions
    /// (see Section 6.4.2.1 on page 177). The resulting value is
    /// (operand * code_alignment_factor).
    pub code_alignment_factor: u128,
    /// A constant that is factored out of certain offset instructions (see
    /// Sections 6.4.2.2 on page 177 and 6.4.2.3 on page 179). The resulting
    /// value is (operand * data_alignment_factor).
    pub data_alignment_factor: i128,
    /// An unsigned LEB128 constant that indicates which column in the rule
    /// table represents the return address of the function. Note that this
    /// column might not correspond to an actual machine register.
    pub return_address_register: u128,
    /// A sequence of rules that are interpreted to create the initial setting
    /// of each column in the table.
    /// The default rule for all columns before interpretation of the initial
    /// instructions is the undefined rule. However, an ABI authoring body
    /// or a compilation system authoring body may specify an alternate
    /// default value for any or all columns.
    pub initial_instructions: &'a [u8],
}

/// Frame Description Entry
#[derive(Debug, PartialEq)]
pub struct Fde<'a> {
    /// A constant that gives the number of bytes of the header and instruction
    /// stream for this function, not including the length field itself (see
    /// Section 7.2.2 on page 184). The size of the length field plus the
    /// value of length must be an integral multiple of the address size.
    pub length: usize,
    /// A constant offset into the .debug_frame section that denotes the CIE
    /// that is associated with this FDE.
    pub cie_pointer: Id,
    /// A 4 byte unsigned value that when subtracted from the offset of the
    /// CIE Pointer in the current FDE yields the offset of the start of the
    /// associated CIE. This value shall never be 0.
    pub pc_begin: usize,
    /// An absolute value that indicates the number of bytes of instructions
    /// associated with this FDE.
    pub pc_range: usize,
    /// The address of the first location associated with this table entry. If
    /// the segment_selector_size field of this FDE’s CIE is non-zero, the
    /// initial location is preceded by a segment selector of the given
    /// length.
    pub initial_location: usize,
    /// The number of bytes of program instructions described by this entry.
    pub address_range: usize,
    /// A sequence of table defining instructions that are described in Section
    /// 6.4.2.
    pub instructions: &'a [u8],
}

#[derive(Debug)]
pub enum Instruction {
    //-------- 6.4.2.1 Row Creation Instructions
    //
    /// The DW_CFA_set_loc instruction takes a single operand that represents a
    /// target address. The required action is to create a new table row using
    /// the specified address as the location. All other values in the new
    /// row are initially identical to the current row. The new location
    /// value is always greater than the current one. If the
    /// segment_selector_size field of this FDE’s CIE is non-zero,
    /// the initial location is preceded by a segment selector of the given
    /// length.
    SetLoc(usize),
    /// The DW_CFA_advance_loc instruction takes a single operand (encoded with
    /// the opcode) that represents a constant delta. The required action is to
    /// create a new table row with a location value that is computed by
    /// taking the current entry’s location value and adding the value of
    /// delta * code_alignment_factor. All other values in the new row are
    /// initially identical to the current row
    AdvanceLoc(u8),
    /// The DW_CFA_advance_loc1 instruction takes a single ubyte operand that
    /// represents a constant delta. This instruction is identical to
    /// DW_CFA_advance_loc except for the encoding and size of the delta operand
    AdvanceLoc1(u8),
    /// The DW_CFA_advance_loc2 instruction takes a single uhalf operand that
    /// represents a constant delta. This instruction is identical to
    /// DW_CFA_advance_loc except for the encoding and size of the delta operand
    AdvanceLoc2(u16),
    /// The DW_CFA_advance_loc4 instruction takes a single uword operand that
    /// represents a constant delta. This instruction is identical to
    /// DW_CFA_advance_loc except for the encoding and size of the delta operand
    AdvanceLoc4(u32),
    //
    //-------- 6.4.2.2 CFA Definition Instructions
    //
    /// The DW_CFA_def_cfa instruction takes two unsigned LEB128 operands
    /// representing a register number and a (non-factored) offset. The required
    /// action is to define the current CFA rule to use the provided register
    /// and offset
    DefCfa {
        register_number: ULeb128,
        offset: ULeb128,
    },
    /// The DW_CFA_def_cfa_sf instruction takes two operands: an unsigned
    /// LEB128 value representing a register number and a signed LEB128 factored
    /// offset. This instruction is identical to DW_CFA_def_cfa except that the
    /// second operand is signed and factored. The resulting offset is
    /// factored_offset * data_alignment_factor.
    DefCfaSf {
        register_number: ULeb128,
        offset: ULeb128,
    },
    /// The DW_CFA_def_cfa_register instruction takes a single unsigned LEB128
    /// operand representing a register number. The required action is to define
    /// the current CFA rule to use the provided register (but to keep the
    /// old offset). This operation is valid only if the current CFA rule is
    /// defined to use a register and offset.
    DefCfaRegister(ULeb128),
    /// The DW_CFA_def_cfa_offset instruction takes a single unsigned LEB128
    /// operand representing a (non-factored) offset. The required action is to
    /// define the current CFA rule to use the provided offset (but to keep
    /// the old register). This operation is valid only if the current CFA
    /// rule is defined to use a register and offset.
    DefCfaOffset(ULeb128),
    /// The DW_CFA_def_cfa_offset_sf instruction takes a signed LEB128 operand
    /// representing a factored offset. This instruction is identical to
    /// DW_CFA_def_cfa_offset except that the operand is signed and factored.
    /// The resulting offset is factored_offset * data_alignment_factor.
    /// This operation is valid only if the current CFA rule is defined to
    /// use a register and offset.
    DefCfaOffsetSf(ULeb128),
    /// The DW_CFA_def_cfa_expression instruction takes a single operand encoded
    /// as a DW_FORM_exprloc value representing a DWARF expression. The
    /// required action is to establish that expression as the means by which
    /// the current CFA is computed.
    DefCfaExpression(Expr),
    //
    //-------- 6.4.2.3 Register Rule Instructions
    //
    /// The DW_CFA_undefined instruction takes a single unsigned LEB128 operand
    /// that represents a register number. The required action is to set the
    /// rule for the specified register to “undefined.”
    Undefined(ULeb128),
    /// The DW_CFA_same_value instruction takes a single unsigned LEB128
    /// operand that represents a register number. The required action is to set
    /// the rule for the specified register to “same value.”
    SameValue(ULeb128),
    /// The DW_CFA_offset instruction takes two operands: a register number
    /// (encoded with the opcode) and an unsigned LEB128 constant representing a
    /// factored offset. The required action is to change the rule for the
    /// register indicated by the register number to be an offset(N) rule
    /// where the value of N is factored offset * data_alignment_factor.
    Offset {
        register_number: usize,
        factored_offset: ULeb128,
    },
    /// The DW_CFA_offset_extended instruction takes two unsigned LEB128
    /// operands representing a register number and a factored offset. This
    /// instruction is identical to DW_CFA_offset except for the encoding and
    /// size of the register operand.
    OffsetExtended {
        register_number: ULeb128,
        factored_offset: ULeb128,
    },
    /// The DW_CFA_offset_extended_sf instruction takes two operands: an
    /// unsigned LEB128 value representing a register number and a signed LEB128
    /// factored offset. This instruction is identical to DW_CFA_offset_extended
    /// except that the second operand is signed and factored. The resulting
    /// offset is factored_offset * data_alignment_factor.
    OffsetExtendedSf {
        register_number: ULeb128,
        factored_offste: ULeb128,
    },
    /// The DW_CFA_val_offset instruction takes two unsigned LEB128 operands
    /// representing a register number and a factored offset. The required
    /// action is to change the rule for the register indicated by the
    /// register number to be a val_offset(N) rule where the value of N is
    /// factored_offset * data_alignment_factor.
    ValOffset {
        register_number: ULeb128,
        factored_offste: ULeb128,
    },
    /// The DW_CFA_val_offset_sf instruction takes two operands: an unsigned
    /// LEB128 value representing a register number and a signed LEB128 factored
    /// offset. This instruction is identical to DW_CFA_val_offset except that
    /// the second operand is signed and factored. The resulting offset is
    /// factored_offset * data_alignment_factor.
    ValOffsetSf {
        register_number: ULeb128,
        factored_offste: ULeb128,
    },
    /// The DW_CFA_register instruction takes two unsigned LEB128 operands
    /// representing register numbers. The required action is to set the rule
    /// for the first register to be register(R) where R is the second
    /// register.
    Register {
        target_register: ULeb128,
        from_register: ULeb128,
    },
    /// The DW_CFA_expression instruction takes two operands: an unsigned
    /// LEB128 value representing a register number, and a DW_FORM_block value
    /// representing a DWARF expression. The required action is to change the
    /// rule for the register indicated by the register number to be an
    /// expression(E) rule where E is the DWARF expression. That is, the
    /// DWARF expression computes the address. The value of the CFA is
    /// pushed on the DWARF evaluation stack prior to execution of the DWARF
    /// expression. See Section 6.4.2 on page 176 regarding restrictions on
    /// the DWARF expression operators that can be used.
    Expression { register: ULeb128, expr: Expr },
    /// The DW_CFA_val_expression instruction takes two operands: an unsigned
    /// LEB128 value representing a register number, and a DW_FORM_block value
    /// representing a DWARF expression. The required action is to change the
    /// rule for the register indicated by the register number to be a
    /// val_expression(E) rule where E is the DWARF expression. That is, the
    /// DWARF expression computes the value of the given register. The value
    /// of the CFA is pushed on the DWARF evaluation stack prior to
    /// execution of the DWARF expression. See Section 6.4.2 on page 176
    /// regarding restrictions on the DWARF expression operators that can be
    /// used.
    ValExpression { register: ULeb128, expr: Expr },
    /// The DW_CFA_restore instruction takes a single operand (encoded with the
    /// opcode) that represents a register number. The required action is to
    /// change the rule for the indicated register to the rule assigned it
    /// by the initial_instructions in the CIE.
    Restore(usize),
    /// The DW_CFA_restore_extended instruction takes a single unsigned LEB128
    /// operand that represents a register number. This instruction is identical
    /// to DW_CFA_restore except for the encoding and size of the register
    /// operand.
    RestoreExtended(ULeb128),
    //
    //-------- 6.4.2.4 Row State Instructions
    //
    /// The DW_CFA_remember_state instruction takes no operands. The required
    /// action is to push the set of rules for every register onto an implicit
    /// stack.
    RememberState,
    /// The DW_CFA_restore_state instruction takes no operands. The required
    /// action is to pop the set of rules off the implicit stack and place them
    /// in the current row.
    RestoreState,
    //
    //-------- 6.4.2.5 Padding Instruction
    //
    /// The DW_CFA_nop instruction has no operands and no required actions. It
    /// is used as padding to make a CIE or FDE an appropriate size.
    Nop,
}

#[derive(Debug, PartialEq)]
enum FrameInfo<'a> {
    Cie(Cie<'a>),
    Fde(Fde<'a>),
}

pub unsafe fn parse_cfi(mut ptr: *const u8) {
    loop {
        ptr = parse_frame_info(ptr).unwrap().2;
    }
}

struct Cursor<'a>(&'a [u8]);

/// Returns `(read_size, value)`
pub(super) unsafe fn read_encoded(
    ptr: *const u8,
    encoding: Encoding,
    datarel_base: Option<usize>,
) -> (usize, usize) {
    let (read_size, value) = match encoding.format() {
        ValueFormat::DW_EH_PE_uleb128 => todo!("uleb128"),
        ValueFormat::DW_EH_PE_udata2 => (2, ptr.cast::<u16>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_udata4 => (4, ptr.cast::<u32>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_udata8 => (8, ptr.cast::<u64>().read_unaligned() as usize),
        ValueFormat::DW_EH_PE_sleb128 => todo!("sleb128"),
        ValueFormat::DW_EH_PE_sdata2 => (2, ptr.cast::<i16>().read_unaligned() as isize as usize),
        ValueFormat::DW_EH_PE_sdata4 => (4, ptr.cast::<i32>().read_unaligned() as isize as usize),
        ValueFormat::DW_EH_PE_sdata8 => (8, ptr.cast::<i64>().read_unaligned() as isize as usize),
    };

    let value = match encoding.application() {
        ValueApplication::DW_EH_PE_absptr => value,
        ValueApplication::DW_EH_PE_pcrel => ((value as isize) + (ptr as isize)) as usize,
        ValueApplication::DW_EH_PE_textrel => todo!("textrel"),
        ValueApplication::DW_EH_PE_datarel => {
            ((value as isize) + (datarel_base.unwrap() as isize)) as usize
        }
        ValueApplication::DW_EH_PE_funcrel => todo!("funcrel"),
        ValueApplication::DW_EH_PE_aligned => todo!("aligned"),
    };

    (read_size, value)
}

#[derive(PartialEq, Clone, Copy)]
#[repr(transparent)]
pub(super) struct Encoding(u8);
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
    pub(crate) fn size(&self) -> usize {
        match self.format() {
            ValueFormat::DW_EH_PE_uleb128 => panic!("uleb128 has no known size"),
            ValueFormat::DW_EH_PE_udata2 => 2,
            ValueFormat::DW_EH_PE_udata4 => 4,
            ValueFormat::DW_EH_PE_udata8 => 8,
            ValueFormat::DW_EH_PE_sleb128 => panic!("sleb128 has no known size"),
            ValueFormat::DW_EH_PE_sdata2 => 2,
            ValueFormat::DW_EH_PE_sdata4 => 4,
            ValueFormat::DW_EH_PE_sdata8 => 8,
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
#[allow(non_camel_case_types)]
enum ValueFormat {
    /// Unsigned value is encoded using the Little Endian Base 128 (LEB128) as
    /// defined by DWARF Debugging Information Format, Revision 2.0.0 (July 27,
    /// 1993).
    DW_EH_PE_uleb128 = 0x01,
    /// A 2 bytes unsigned value.
    DW_EH_PE_udata2 = 0x02,
    /// A 4 bytes unsigned value.
    DW_EH_PE_udata4 = 0x03,
    /// An 8 bytes unsigned value.
    DW_EH_PE_udata8 = 0x04,
    /// Signed value is encoded using the Little Endian Base 128 (LEB128) as
    /// defined by DWARF Debugging Information Format, Revision 2.0.0 (July 27,
    /// 1993).
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
#[allow(non_camel_case_types)]
enum ValueApplication {
    DW_EH_PE_absptr = 0x00,
    ///	Value is relative to the current program counter.
    DW_EH_PE_pcrel = 0x10,
    ///	Value is relative to the beginning of the .text section.
    DW_EH_PE_textrel = 0x20,
    ///	Value is relative to the beginning of the .got or .eh_frame_hdr
    /// section.
    DW_EH_PE_datarel = 0x30,
    ///	Value is relative to the beginning of the function.
    DW_EH_PE_funcrel = 0x40,
    ///	Value is aligned to an address unit sized boundary.
    DW_EH_PE_aligned = 0x50,
}

fn read_bytes<'a>(data: &mut Cursor<'a>, amount: usize) -> Result<&'a [u8]> {
    if data.0.len() < amount {
        return Err(Error(format!(
            "index out of bounds, tried to read {amount} bytes from {}",
            data.0.len()
        )));
    } else {
        let result = &data.0[..amount];
        data.0 = &data.0[amount..];
        Ok(result)
    }
}
fn read_u32(data: &mut Cursor<'_>) -> Result<u32> {
    let int = read_bytes(data, 4)?;
    Ok(u32::from_le_bytes(int.try_into().unwrap()))
}
fn read_u8(data: &mut Cursor<'_>) -> Result<u8> {
    let int = read_bytes(data, 1)?;
    Ok(int[0])
}
fn read_utf8_cstr<'a>(data: &mut Cursor<'a>) -> Result<&'a str> {
    let cstr: &CStr = CStr::from_bytes_until_nul(data.0)
        .map_err(|_| Error("no null terminator found for string".into()))?;
    let utf8 = cstr
        .to_str()
        .map_err(|e| Error(format!("invalid utf8: {e:?}")))?;
    data.0 = &data.0[(utf8.len() + 1)..];
    Ok(utf8)
}
fn read_uleb128(data: &mut Cursor<'_>) -> Result<u128> {
    let mut result = 0;
    let mut shift = 0;
    loop {
        let byte = read_u8(data)?;
        result |= ((byte & 0b0111_1111) << shift) as u128;
        if (byte >> 7) == 0 {
            break;
        }
        shift += 7;
    }
    Ok(result)
}
fn read_ileb128(data: &mut Cursor<'_>) -> Result<i128> {
    let mut result = 0;
    let mut shift = 0;
    let size = 8;

    let sign_bit_set = loop {
        let byte = read_u8(data)?;
        result |= ((byte & 0b0111_1111) << shift) as i128;
        shift += 7;
        if (byte >> 7) == 0 {
            let sign_bit_set = ((byte >> 6) & 1) == 1;
            break sign_bit_set;
        }
    };
    if (shift < size) && sign_bit_set {
        result |= -(1 << shift);
    }
    Ok(result)
}

#[instrument(ret)]
unsafe fn parse_frame_info<'a>(
    ptr: *const u8,
) -> Result<(Cie<'a>, alloc::vec::Vec<Fde<'a>>, *const u8)> {
    let (cie_id, data, mut prev_ptr) = parse_frame_head(ptr)?;
    if cie_id != 0 {
        return Err(Error(format!("CIE must have cie_id=0")));
    }
    let data = &mut Cursor(data);
    let cie = parse_cie(data)?;

    let mut fdes = alloc::vec::Vec::new();

    loop {
        let (cie_id, data, newer_ptr) = parse_frame_head(ptr)?;
        trace!("cie id: {cie_id}");
        if cie_id != 0 {
            return Ok((cie, fdes, prev_ptr));
        }
        prev_ptr = newer_ptr;
        let data = &mut Cursor(data);
        let fde = parse_fde(data, cie_id, &cie)?;
        trace!("FDE: {fde:?}");
        fdes.push(fde);
    }
}

unsafe fn parse_frame_head<'a>(ptr: *const u8) -> Result<(u32, &'a [u8], *const u8)> {
    let len = ptr.cast::<u32>().read();
    if len == 0xffffffff {
        // be careful, if you handle this you need to adjust the callers offsets lol lmao
        todo!("loooong dwarf, cannot handle.");
    }
    let data = &mut Cursor(core::slice::from_raw_parts(ptr.add(4), len as usize));
    trace!("frame info entry (without len): {:x?}", data.0);

    let cie_id = read_u32(data)?;
    let new_ptr = ptr.add(4).add(len as _);

    Ok((cie_id, data.0, new_ptr))
}

#[instrument(skip(data))]
fn parse_cie<'a>(data: &mut Cursor<'a>) -> Result<Cie<'a>> {
    let version = read_u8(data)?;
    if version != 1 {
        return Err(Error(format!("version must be 1: {version}")));
    }

    let augmentation = read_utf8_cstr(data)?;
    let code_alignment_factor = read_uleb128(data)?;
    let data_alignment_factor = read_ileb128(data)?;
    let return_address_register = read_uleb128(data)?;

    let augmentation_data = if augmentation.starts_with('z') {
        let aug_len = read_uleb128(data)?;
        let aug_data = read_bytes(data, aug_len as usize)?;

        let aug = parse_augmentation_data(augmentation, aug_data)?;
        trace!("AUGMENTATION {aug:?}");

        Some(aug)
    } else {
        None
    };

    let initial_instructions = data.0;

    let cie = Cie {
        augmentation: augmentation_data,
        augmentation_string: augmentation,
        code_alignment_factor,
        data_alignment_factor,
        return_address_register,
        initial_instructions,
    };

    trace!("{cie:?}");
    Ok(cie)
}

pub(crate) unsafe fn parse_fde_from_ptr<'a>(
    ptr: *const u8,
    eh_frame_base: usize,
) -> Result<Fde<'a>> {
    let (fde_cie_id, fde_data, _) = parse_frame_head(ptr)?;
    let fde_data = &mut Cursor(fde_data);

    if fde_cie_id == 0 {
        return Err(Error(format!("FDE's CIE Pointer is 0")));
    }
    trace!("FDE's CIE pointer: {fde_cie_id}");

    let cie_ptr = ptr.byte_add(4 /* length */).byte_sub(fde_cie_id as usize);

    trace!(
        "CIE offset to .eh_frame: {:x}",
        cie_ptr.addr() - (eh_frame_base)
    );

    let (cie_cie_id, cie_data, _) = parse_frame_head(cie_ptr)?;
    if cie_cie_id != 0 {
        return Err(Error(format!("CIE must have cie_id=0")));
    }
    let cie_data = &mut Cursor(cie_data);
    let cie = parse_cie(cie_data)?;

    let fde = parse_fde(fde_data, fde_cie_id, &cie)?;

    Ok(fde)
}

#[instrument(skip(data))]
fn parse_fde<'a>(data: &mut Cursor<'a>, cie_id: u32, cie: &Cie<'_>) -> Result<Fde<'a>> {
    trace!("FDE {:x?}", data.0);

    let augmentation = cie
        .augmentation
        .as_ref()
        .ok_or_else(|| Error("augmentation data not present for CIE with FDEs".into()))?;

    trace!("augmentation: {augmentation:?}");

    let pointer_encoding = augmentation.pointer_encoding.ok_or_else(|| {
        Error("pointer encoding not present in augmentation for CIE with FDEs".into())
    })?;

    let (read_size, pc_begin) = unsafe { read_encoded(data.0.as_ptr(), pointer_encoding, None) };
    data.0 = &data.0[read_size..];

    let (read_size, pc_range) = unsafe { read_encoded(data.0.as_ptr(), pointer_encoding, None) };
    data.0 = &data.0[read_size..];

    // This is only present if the aug data of the CIE contains z. But that is
    // always the case?
    let augmentation_len = read_uleb128(data)?;
    let augmentation_data = &data.0[..(augmentation_len as usize)];
    let data = parse_augmentation_data(cie.augmentation_string, augmentation_data)?;
    trace!("debug data: {data:?}");

    Err(Error("aa".into()))
}

#[derive(Debug, PartialEq)]
pub struct AugmentationData {
    pub(super) lsda_pointer_encoding: Option<Encoding>,
    pub(super) pointer_encoding: Option<Encoding>,
    pub(super) personality: Option<usize>,
}

fn parse_augmentation_data(string: &str, data: &[u8]) -> Result<AugmentationData> {
    let data = &mut Cursor(data);

    let mut codes = string.bytes();
    assert_eq!(codes.next(), Some(b'z'));
    trace!("aug data {:?} | {:x?}", string, data.0);

    let mut aug_data = AugmentationData {
        pointer_encoding: None,
        lsda_pointer_encoding: None,
        personality: None,
    };

    for code in codes {
        match code {
            // If present, it indicates the presence of one argument in the Augmentation Data of the
            // CIE, and a corresponding argument in the Augmentation Data of the FDE.
            // The argument in the Augmentation Data of the CIE is 1-byte and represents the pointer
            // encoding used for the argument in the Augmentation Data of the FDE, which
            // is the address of a language-specific data area (LSDA). The size of the
            // LSDA pointer is specified by the pointer encoding used.
            b'L' => {
                trace!("L");
                let encoding = read_u8(data)?;
                aug_data.lsda_pointer_encoding = Some(Encoding(encoding));
            }
            // If present, it indicates the presence of two arguments in the Augmentation Data of
            // the CIE. The first argument is 1-byte and represents the pointer encoding
            // used for the second argument, which is the address of a personality
            // routine handler. The personality routine is used to handle language and
            // vendor-specific tasks. The system unwind library interface accesses the
            // language-specific exception handling semantics via the pointer to the
            // personality routine. The personality routine does not
            // have an ABI-specific name. The size of the personality routine pointer is specified
            // by the pointer encoding used.
            b'P' => {
                trace!("P");
                let encoding = Encoding(read_u8(data)?);
                let (read_size, value) = unsafe { read_encoded(data.0.as_ptr(), encoding, None) };
                data.0 = &data.0[read_size..];
                aug_data.personality = Some(value);
            }
            // If present, The Augmentation Data shall include a 1 byte argument that represents the
            // pointer encoding for the address pointers used in the FDE.
            b'R' => {
                let encoding = read_u8(data)?;
                aug_data.pointer_encoding = Some(Encoding(encoding));
            }
            _ => return Err(Error(format!("invalid augmentation code: {code}"))),
        }
    }

    Ok(aug_data)
}

pub(super) struct InstrIter {
    data: *const u8,
}

impl InstrIter {
    /// Create a new `InstrIter` that will parse DWARF call frame information
    /// from `data`. # Safety
    /// `data` must be a pointer to valid DWARF call frame information with a
    /// null terminator.
    pub(super) unsafe fn new(data: *const u8) -> Self {
        // SAFETY: uses random ass pointer
        Self { data }
    }

    fn advance(&mut self) -> Option<u8> {
        // SAFETY: First, we assume that `data` currently points at a valid location.
        // After we read from it, we increment it. This has implications. We must assume
        // that the dwarf parsing code in this module never calls `advance` more than it
        // has to. This means that really `advance` should be unsafe, but
        // marking it as unsafe would only make the code in here harder to read
        // and not provide practical safety improvements. We do eagerly move the
        // data pointer outside the bounds of the allocation, but only one
        // past the end, which is fine.
        unsafe {
            let first = self.data.read();
            self.data = self.data.add(1);
            Some(first)
        }
    }

    fn uleb128(&mut self) -> ULeb128 {
        ULeb128::parse()
    }
}

const DW_CFA_advance_loc_hi: u8 = 0x01;
const DW_CFA_offset_hi: u8 = 0x02;
const DW_CFA_restore_hi: u8 = 0x03;

const DW_CFA_nop: u8 = 0;
const DW_CFA_set_loc: u8 = 0x01;
const DW_CFA_advance_loc1: u8 = 0x02;
const DW_CFA_advance_loc2: u8 = 0x03;
const DW_CFA_advance_loc4: u8 = 0x04;
const DW_CFA_offset_extended: u8 = 0x05;
const DW_CFA_restore_extended: u8 = 0x06;
const DW_CFA_undefined: u8 = 0x07;
const DW_CFA_same_value: u8 = 0x08;
const DW_CFA_register: u8 = 0x09;
const DW_CFA_remember_state: u8 = 0x0a;
const DW_CFA_restore_state: u8 = 0x0b;
const DW_CFA_def_cfa: u8 = 0x0c;
const DW_CFA_def_cfa_register: u8 = 0x0d;
const DW_CFA_def_cfa_offset: u8 = 0x0e;
const DW_CFA_def_cfa_expression: u8 = 0x0f;
const DW_CFA_expression: u8 = 0x10;
const DW_CFA_offset_extended_sf: u8 = 0x11;
const DW_CFA_def_cfa_sf: u8 = 0x12;
const DW_CFA_def_cfa_offset_sf: u8 = 0x13;
const DW_CFA_val_offset: u8 = 0x14;
const DW_CFA_val_offset_sf: u8 = 0x15;
const DW_CFA_val_expression: u8 = 0x16;
const DW_CFA_lo_user: u8 = 0x1c;
const DW_CFA_hi_user: u8 = 0x3f;

impl Iterator for InstrIter {
    type Item = Instruction;

    #[allow(unreachable_code)]
    fn next(&mut self) -> Option<Self::Item> {
        let b = self.advance()?;
        let high_2 = b & !(u8::MAX >> 2);
        Some(match high_2 {
            DW_CFA_advance_loc_hi => {
                let delta = b & (u8::MAX >> 2);
                Instruction::AdvanceLoc(delta)
            }
            DW_CFA_offset_hi => {
                let register = b & (u8::MAX >> 2);
                Instruction::Offset {
                    register_number: register as _,
                    factored_offset: self.uleb128(),
                }
            }
            DW_CFA_restore_hi => {
                let register = b & (u8::MAX >> 2);
                Instruction::Restore(register as _)
            }
            _ => match b {
                DW_CFA_nop => Instruction::Nop,
                DW_CFA_set_loc => Instruction::SetLoc(todo!()),
                DW_CFA_advance_loc1 => Instruction::AdvanceLoc1(todo!()),
                DW_CFA_advance_loc2 => Instruction::AdvanceLoc2(todo!()),
                DW_CFA_advance_loc4 => Instruction::AdvanceLoc4(todo!()),
                DW_CFA_offset_extended => Instruction::OffsetExtended {
                    register_number: self.uleb128(),
                    factored_offset: self.uleb128(),
                },
                DW_CFA_restore_extended => Instruction::RestoreExtended(self.uleb128()),
                DW_CFA_undefined => Instruction::Undefined(self.uleb128()),
                DW_CFA_same_value => Instruction::SameValue(self.uleb128()),
                DW_CFA_register => Instruction::Register {
                    target_register: self.uleb128(),
                    from_register: self.uleb128(),
                },
                DW_CFA_remember_state => Instruction::RememberState,
                DW_CFA_restore_state => Instruction::RestoreState,
                DW_CFA_def_cfa => Instruction::DefCfa {
                    register_number: self.uleb128(),
                    offset: self.uleb128(),
                },
                DW_CFA_def_cfa_register => Instruction::DefCfaRegister(self.uleb128()),
                DW_CFA_def_cfa_offset => Instruction::DefCfaOffset(self.uleb128()),
                DW_CFA_def_cfa_expression => Instruction::DefCfaExpression(todo!()),
                DW_CFA_expression => Instruction::Expression {
                    register: self.uleb128(),
                    expr: todo!(),
                },
                DW_CFA_offset_extended_sf => Instruction::OffsetExtendedSf {
                    register_number: self.uleb128(),
                    factored_offste: self.uleb128(),
                },
                DW_CFA_def_cfa_sf => Instruction::DefCfaSf {
                    register_number: self.uleb128(),
                    offset: self.uleb128(),
                },
                DW_CFA_def_cfa_offset_sf => Instruction::DefCfaOffsetSf(self.uleb128()),
                DW_CFA_val_offset => Instruction::ValOffset {
                    register_number: self.uleb128(),
                    factored_offste: self.uleb128(),
                },
                DW_CFA_val_offset_sf => Instruction::ValOffsetSf {
                    register_number: self.uleb128(),
                    factored_offste: self.uleb128(),
                },
                DW_CFA_val_expression => Instruction::ValExpression {
                    register: self.uleb128(),
                    expr: todo!(),
                },
                _ => todo!(),
            },
        })
    }
}
