//! ISA 模块测试

use super::*;

#[test]
fn test_decode_addi() {
    let raw = 0x02A00093; // addi x1, x0, 42
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Addi {
            rd: 1,
            rs1: 0,
            imm: 42
        }
    );
}

#[test]
fn test_decode_addi_negative() {
    let raw = 0xFFF00113; // addi x2, x0, -1
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Addi {
            rd: 2,
            rs1: 0,
            imm: -1
        }
    );
}

#[test]
fn test_decode_add() {
    let raw = 0x002081B3; // add x3, x1, x2
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Add {
            rd: 3,
            rs1: 1,
            rs2: 2
        }
    );
}

#[test]
fn test_decode_sub() {
    let raw = 0x402081B3; // sub x3, x1, x2
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Sub {
            rd: 3,
            rs1: 1,
            rs2: 2
        }
    );
}

#[test]
fn test_decode_lw() {
    let raw = 0x00412083; // lw x1, 4(x2)
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Lw {
            rd: 1,
            rs1: 2,
            offset: 4
        }
    );
}

#[test]
fn test_decode_sw() {
    let raw = 0x00112423; // sw x1, 8(x2)
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Sw {
            rs1: 2,
            rs2: 1,
            offset: 8
        }
    );
}

#[test]
fn test_decode_beq() {
    let raw = 0x00208463; // beq x1, x2, 8
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Beq {
            rs1: 1,
            rs2: 2,
            offset: 8
        }
    );
}

#[test]
fn test_decode_jal() {
    let raw = 0x000000EF; // jal x1, 0
    let decoded = decode(raw);
    assert_eq!(decoded.instr, RvInstr::Jal { rd: 1, offset: 0 });
}

#[test]
fn test_decode_lui() {
    let raw = 0x123450B7; // lui x1, 0x12345
    let decoded = decode(raw);
    assert_eq!(
        decoded.instr,
        RvInstr::Lui {
            rd: 1,
            imm: 0x12345000_u32 as i32
        }
    );
}

#[test]
fn test_decode_ecall() {
    let raw = 0x00000073;
    let decoded = decode(raw);
    assert_eq!(decoded.instr, RvInstr::Ecall);
}

#[test]
fn test_decode_ebreak() {
    let raw = 0x00100073;
    let decoded = decode(raw);
    assert_eq!(decoded.instr, RvInstr::Ebreak);
}

#[test]
fn test_decode_illegal() {
    let raw = 0x00000000;
    let decoded = decode(raw);
    match decoded.instr {
        RvInstr::Illegal { raw: r } => assert_eq!(r, 0),
        _ => panic!("Expected Illegal instruction"),
    }
}

#[test]
fn test_decoder_registry() {
    let registry = DecoderRegistry::with_rv32i();
    
    // 测试正常解码
    let decoded = registry.decode(0x02A00093); // addi x1, x0, 42
    assert_eq!(
        decoded.instr,
        RvInstr::Addi {
            rd: 1,
            rs1: 0,
            imm: 42
        }
    );
    
    // 测试非法指令
    let decoded = registry.decode(0x00000000);
    assert!(matches!(decoded.instr, RvInstr::Illegal { .. }));
}

#[test]
fn test_decoder_registry_multiple_decoders() {
    use std::sync::Arc;
    
    // 创建一个自定义解码器，处理 custom-0 opcode
    struct CustomDecoder;
    
    impl InstrDecoder for CustomDecoder {
        fn name(&self) -> &str {
            "Custom"
        }
        
        fn decode(&self, raw: u32) -> Option<DecodedInstr> {
            if (raw & 0x7F) == 0b0001011 {
                Some(DecodedInstr {
                    raw,
                    instr: RvInstr::Custom {
                        extension: "test",
                        opcode: 0b0001011,
                        raw,
                        fields: CustomFields::new()
                            .with_rd(rd(raw))
                            .with_rs1(rs1(raw)),
                    },
                })
            } else {
                None
            }
        }
        
        fn handled_opcodes(&self) -> Option<&[u32]> {
            static OPS: [u32; 1] = [0b0001011];
            Some(&OPS)
        }
    }
    
    let mut registry = DecoderRegistry::with_rv32i();
    registry
        .register(Arc::new(CustomDecoder))
        .expect("custom decoder should register");
    // 再次注册相同 opcode 应该失败
    assert!(registry.register(Arc::new(CustomDecoder)).is_err());
    assert_eq!(registry.decoder_count(), 2);
    
    // 测试自定义指令解码
    let custom_raw = 0x0000000B; // opcode = 0b0001011
    let decoded = registry.decode(custom_raw);
    assert!(matches!(decoded.instr, RvInstr::Custom { extension: "test", .. }));
    
    // RV32I 指令仍然正常工作
    let decoded = registry.decode(0x02A00093);
    assert!(matches!(decoded.instr, RvInstr::Addi { .. }));
}

#[test]
fn test_custom_fields() {
    let fields = CustomFields::new()
        .with_rd(5)
        .with_rs1(10)
        .with_rs2(15)
        .with_imm(-100)
        .with_extra(0xDEADBEEF);
    
    assert_eq!(fields.rd, Some(5));
    assert_eq!(fields.rs1, Some(10));
    assert_eq!(fields.rs2, Some(15));
    assert_eq!(fields.imm, Some(-100));
    assert_eq!(fields.extra, 0xDEADBEEF);
}
