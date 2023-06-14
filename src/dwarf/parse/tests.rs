use crate::dwarf::parse::Cie;

#[test]
fn parse_simple_cie() {
    #[rustfmt::skip]
    let data = [
        0x14, 0, 0, 0,
        0, 0, 0, 0, 1,
        0x7a, 0x52, 0, 1,
        0x78, 0x10, 1,
        0x1b, 0xc, 7, 8,
        0x90, 1, 0, 0,
    ];

    let cie = unsafe { super::parse_cie(data.as_ptr()) }.unwrap();

    assert_eq!(
        cie,
        Cie {
            augmentation: "zR",
            code_alignment_factor: 1,
            data_alignment_factor: -8,
            return_address_register: 16,
            augmentation_data: Some(b"\x1B"),
            initial_instructions: &[0xc, 7, 8, 0x90, 1, 0, 0]
        }
    );

    // llvm-dwarfdump output:
    /*
    00000000 00000014 00000000 CIE
    Format:                DWARF32
    Version:               1
    Augmentation:          "zR"
    Code alignment factor: 1
    Data alignment factor: -8
    Return address column: 16
    Augmentation data:     1B

    DW_CFA_def_cfa: RSP +8
    DW_CFA_offset: RIP -8
    DW_CFA_nop:
    DW_CFA_nop:

    CFA=RSP+8: RIP=[CFA-8]
    */
}
