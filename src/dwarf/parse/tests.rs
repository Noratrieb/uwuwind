use crate::dwarf::parse::{AugmentationData, Cie, Encoding, ValueApplication, ValueFormat};

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

    let cie = unsafe { super::parse_frame_info(data.as_ptr()) }.unwrap().0;

    assert_eq!(
        cie,
        Cie {
            augmentation: Some(AugmentationData {
                lsda_pointer_encoding: None,
                pointer_encoding: Some(Encoding(
                    (ValueApplication::DW_EH_PE_pcrel as u8) | (ValueFormat::DW_EH_PE_sdata4 as u8)
                )),
                personality: None
            }),
            code_alignment_factor: 1,
            data_alignment_factor: -8,
            return_address_register: 16,
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
