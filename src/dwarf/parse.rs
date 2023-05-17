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
//! The first column is the address for every location that contains code in a program
//! (a relative offset in shared object files). The remaining columns contain unwinding rules
//! that are associated with the indicated location.
//!
//! The CFA column defines the rule which computes the Canonical Frame Address
//! value; it may be either a register and a signed offset that are added together, or a
//! DWARF expression that is evaluated.
//!
//! The remaining columns describe register numbers that indicate whether a register has been saved
//! and the rule to find the value for the previous frame.
#![allow(non_upper_case_globals)]

struct Expr;

enum RegisterRule {
    /// A register that has this rule has no recoverable value in the previous frame.
    /// (By convention, it is not preserved by a callee.)
    Undefined,
    /// This register has not been modified from the previous frame.
    /// (By convention, it is preserved by the callee, but the callee has not modified it.)
    SameValue,
    /// The previous value of this register is saved at the address CFA+N where CFA
    /// is the current CFA value and N is a signed offset
    Offset(isize),
    /// The previous value of this register is the value CFA+N where CFA is the current CFA value
    /// and N is a signed offset.
    ValOffset(isize),
    /// The previous value of this register is stored in another register numbered R.
    Register(u16),
    /// The previous value of this register is located at the address produced by
    /// executing the DWARF expression E (see Section 2.5 on page 26)
    Expression(Expr),
    /// The previous value of this register is the value produced by executing the DWARF
    /// expression E (see Section 2.5 on page 26).
    ValExpression(Expr),
    ///  The rule is defined externally to this specification by the augmenter.
    Architectural,
}

type Id = u32;

struct ULeb128(u128);
impl ULeb128 {
    fn parse() -> Self {
        todo!()
    }
}
struct ILeb128(i128);
impl ILeb128 {
    fn parse() -> Self {
        todo!()
    }
}

/// Common Information Entry
struct Cie<'a> {
    /// A constant that gives the number of bytes of the CIE structure, not including
    /// the length field itself (see Section 7.2.2 on page 184). The size of the length
    /// field plus the value of length must be an integral multiple of the address size.
    length: usize,
    /// A constant that is used to distinguish CIEs from FDEs.
    cie_id: Id,
    /// A version number (see Section 7.24 on page 238). This number is specific to
    /// the call frame information and is independent of the DWARF version number.
    version: u8,
    /// A null-terminated UTF-8 string that identifies the augmentation to this CIE or
    /// to the FDEs that use it. If a reader encounters an augmentation string that is
    /// unexpected, then only the following fields can be read:
    /// - CIE: length, CIE_id, version, augmentation
    /// - FDE: length, CIE_pointer, initial_location, address_range
    ///
    /// If there is no augmentation, this value is a zero byte.
    ///
    /// The augmentation string allows users to indicate that there is additional
    /// target-specific information in the CIE or FDE which is needed to virtually unwind a
    /// stack frame. For example, this might be information about dynamically allocated data
    /// which needs to be freed on exit from the routine.
    ///
    /// Because the .debug_frame section is useful independently of any .debug_info
    /// section, the augmentation string always uses UTF-8 encoding.
    augmentation: &'a str,
    /// The size of a target address in this CIE and any FDEs that use it, in bytes. If a
    /// compilation unit exists for this frame, its address size must match the address
    /// size here.
    address_size: u8,
    /// The size of a segment selector in this CIE and any FDEs that use it, in bytes.
    segment_selector_size: u8,
    /// A constant that is factored out of all advance location instructions (see
    /// Section 6.4.2.1 on page 177). The resulting value is
    /// (operand * code_alignment_factor).
    code_alignment_factor: ULeb128,
    /// A constant that is factored out of certain offset instructions (see
    /// Sections 6.4.2.2 on page 177 and 6.4.2.3 on page 179). The resulting value is
    /// (operand * data_alignment_factor).
    data_alignment_factor: ILeb128,
    /// An unsigned LEB128 constant that indicates which column in the rule table
    /// represents the return address of the function. Note that this column might not
    /// correspond to an actual machine register.
    return_address_register: ULeb128,
    /// A sequence of rules that are interpreted to create the initial setting of each
    /// column in the table.
    /// The default rule for all columns before interpretation of the initial instructions
    /// is the undefined rule. However, an ABI authoring body or a compilation
    /// system authoring body may specify an alternate default value for any or all
    /// columns.
    initial_instructions: &'a [u8],
}

/// Frame Description Entry
struct Fde<'a> {
    /// A constant that gives the number of bytes of the header and instruction
    /// stream for this function, not including the length field itself (see Section 7.2.2
    /// on page 184). The size of the length field plus the value of length must be an
    /// integral multiple of the address size.
    length: usize,
    /// A constant offset into the .debug_frame section that denotes the CIE that is
    /// associated with this FDE.
    cie_pointer: Id,
    /// The address of the first location associated with this table entry. If the
    /// segment_selector_size field of this FDE’s CIE is non-zero, the initial
    /// location is preceded by a segment selector of the given length.
    initial_location: usize,
    /// The number of bytes of program instructions described by this entry.
    address_range: usize,
    /// A sequence of table defining instructions that are described in Section 6.4.2.
    instructions: &'a [u8],
}

enum Instruction {
    //-------- 6.4.2.1 Row Creation Instructions
    //
    /// The DW_CFA_set_loc instruction takes a single operand that represents a
    /// target address. The required action is to create a new table row using the
    /// specified address as the location. All other values in the new row are initially
    /// identical to the current row. The new location value is always greater than the
    /// current one. If the segment_selector_size field of this FDE’s CIE is non-zero,
    /// the initial location is preceded by a segment selector of the given length.
    SetLoc(usize),
    /// The DW_CFA_advance_loc instruction takes a single operand (encoded with
    /// the opcode) that represents a constant delta. The required action is to create a
    /// new table row with a location value that is computed by taking the current
    /// entry’s location value and adding the value of delta * code_alignment_factor.
    /// All other values in the new row are initially identical to the current row
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
    /// action is to define the current CFA rule to use the provided register and offset
    DefCfa {
        register_number: ULeb128,
        offset: ULeb128,
    },
    /// The DW_CFA_def_cfa_sf instruction takes two operands: an unsigned
    /// LEB128 value representing a register number and a signed LEB128 factored
    /// offset. This instruction is identical to DW_CFA_def_cfa except that the second
    /// operand is signed and factored. The resulting offset is factored_offset *
    /// data_alignment_factor.
    DefCfaSf {
        register_number: ULeb128,
        offset: ULeb128,
    },
    /// The DW_CFA_def_cfa_register instruction takes a single unsigned LEB128
    /// operand representing a register number. The required action is to define the
    /// current CFA rule to use the provided register (but to keep the old offset). This
    /// operation is valid only if the current CFA rule is defined to use a register and
    /// offset.
    DefCfaRegister(ULeb128),
    /// The DW_CFA_def_cfa_offset instruction takes a single unsigned LEB128
    /// operand representing a (non-factored) offset. The required action is to define
    /// the current CFA rule to use the provided offset (but to keep the old register).
    /// This operation is valid only if the current CFA rule is defined to use a register
    /// and offset.
    DefCfaOffset(ULeb128),
    /// The DW_CFA_def_cfa_offset_sf instruction takes a signed LEB128 operand
    /// representing a factored offset. This instruction is identical to
    /// DW_CFA_def_cfa_offset except that the operand is signed and factored. The
    /// resulting offset is factored_offset * data_alignment_factor. This operation is
    /// valid only if the current CFA rule is defined to use a register and offset.
    DefCfaOffsetSf(ULeb128),
    /// The DW_CFA_def_cfa_expression instruction takes a single operand encoded
    /// as a DW_FORM_exprloc value representing a DWARF expression. The
    /// required action is to establish that expression as the means by which the
    /// current CFA is computed.
    DefCfaExpression(Expr),
    //
    //-------- 6.4.2.3 Register Rule Instructions
    //
    /// The DW_CFA_undefined instruction takes a single unsigned LEB128 operand
    /// that represents a register number. The required action is to set the rule for the
    /// specified register to “undefined.”
    Undefined(ULeb128),
    /// The DW_CFA_same_value instruction takes a single unsigned LEB128
    /// operand that represents a register number. The required action is to set the
    /// rule for the specified register to “same value.”
    SameValue(ULeb128),
    /// The DW_CFA_offset instruction takes two operands: a register number
    /// (encoded with the opcode) and an unsigned LEB128 constant representing a
    /// factored offset. The required action is to change the rule for the register
    /// indicated by the register number to be an offset(N) rule where the value of N
    /// is factored offset * data_alignment_factor.
    Offset {
        register_number: usize,
        factored_offset: ULeb128,
    },
    /// The DW_CFA_offset_extended instruction takes two unsigned LEB128
    /// operands representing a register number and a factored offset. This
    /// instruction is identical to DW_CFA_offset except for the encoding and size of
    /// the register operand.
    OffsetExtended {
        register_number: ULeb128,
        factored_offset: ULeb128,
    },
    /// The DW_CFA_offset_extended_sf instruction takes two operands: an
    /// unsigned LEB128 value representing a register number and a signed LEB128
    /// factored offset. This instruction is identical to DW_CFA_offset_extended
    /// except that the second operand is signed and factored. The resulting offset is
    /// factored_offset * data_alignment_factor.
    OffsetExtendedSf {
        register_number: ULeb128,
        factored_offste: ULeb128,
    },
    /// The DW_CFA_val_offset instruction takes two unsigned LEB128 operands
    /// representing a register number and a factored offset. The required action is to
    /// change the rule for the register indicated by the register number to be a
    /// val_offset(N) rule where the value of N is factored_offset *
    /// data_alignment_factor.
    ValOffset {
        register_number: ULeb128,
        factored_offste: ULeb128,
    },
    /// The DW_CFA_val_offset_sf instruction takes two operands: an unsigned
    /// LEB128 value representing a register number and a signed LEB128 factored
    /// offset. This instruction is identical to DW_CFA_val_offset except that the
    /// second operand is signed and factored. The resulting offset is factored_offset *
    /// data_alignment_factor.
    ValOffsetSf {
        register_number: ULeb128,
        factored_offste: ULeb128,
    },
    /// The DW_CFA_register instruction takes two unsigned LEB128 operands
    /// representing register numbers. The required action is to set the rule for the
    /// first register to be register(R) where R is the second register.
    Register {
        target_register: ULeb128,
        from_register: ULeb128,
    },
    /// The DW_CFA_expression instruction takes two operands: an unsigned
    /// LEB128 value representing a register number, and a DW_FORM_block value
    /// representing a DWARF expression. The required action is to change the rule
    /// for the register indicated by the register number to be an expression(E) rule
    /// where E is the DWARF expression. That is, the DWARF expression computes
    /// the address. The value of the CFA is pushed on the DWARF evaluation stack
    /// prior to execution of the DWARF expression.
    /// See Section 6.4.2 on page 176 regarding restrictions on the DWARF expression
    /// operators that can be used.
    Expression { register: ULeb128, expr: Expr },
    /// The DW_CFA_val_expression instruction takes two operands: an unsigned
    /// LEB128 value representing a register number, and a DW_FORM_block value
    /// representing a DWARF expression. The required action is to change the rule
    /// for the register indicated by the register number to be a val_expression(E)
    /// rule where E is the DWARF expression. That is, the DWARF expression
    /// computes the value of the given register. The value of the CFA is pushed on
    /// the DWARF evaluation stack prior to execution of the DWARF expression.
    /// See Section 6.4.2 on page 176 regarding restrictions on the DWARF expression
    /// operators that can be used.
    ValExpression { register: ULeb128, expr: Expr },
    /// The DW_CFA_restore instruction takes a single operand (encoded with the
    /// opcode) that represents a register number. The required action is to change
    /// the rule for the indicated register to the rule assigned it by the
    /// initial_instructions in the CIE.
    Restore(usize),
    /// The DW_CFA_restore_extended instruction takes a single unsigned LEB128
    /// operand that represents a register number. This instruction is identical to
    /// DW_CFA_restore except for the encoding and size of the register operand.
    RestoreExtended(ULeb128),
    //
    //-------- 6.4.2.4 Row State Instructions
    //
    /// The DW_CFA_remember_state instruction takes no operands. The required
    /// action is to push the set of rules for every register onto an implicit stack.
    RememberState,
    /// The DW_CFA_restore_state instruction takes no operands. The required
    /// action is to pop the set of rules off the implicit stack and place them in the
    /// current row.
    RestoreState,
    //
    //-------- 6.4.2.5 Padding Instruction
    //
    /// The DW_CFA_nop instruction has no operands and no required actions. It is
    /// used as padding to make a CIE or FDE an appropriate size.
    Nop,
}

struct InstrIter<'a> {
    data: &'a [u8],
}

impl<'a> InstrIter<'a> {
    fn advance(&mut self) -> Option<u8> {
        let (&first, rest) = self.data.split_first()?;
        self.data = rest;
        Some(first)
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

impl<'a> Iterator for InstrIter<'a> {
    type Item = Instruction;

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
