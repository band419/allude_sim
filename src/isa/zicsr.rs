//! Zicsr 扩展（CSR 操作指令）解码器
//!
//! 包含 6 条 CSR 操作指令：
//! - CSRRW, CSRRS, CSRRC (寄存器版本)
//! - CSRRWI, CSRRSI, CSRRCI (立即数版本)

use crate::isa::fields::*;
use crate::isa::instr::RvInstr;
use crate::isa::instr_def::{InstrDef, TableDrivenDecoder, I_TYPE_MASK, i_match};

// ========== Zicsr 指令定义表 ==========

/// Zicsr 指令定义表
/// 
/// CSR 指令编码格式（I-type 变体）：
/// ```text
/// 31       20 19    15 14  12 11   7 6      0
/// ┌──────────┬────────┬──────┬──────┬────────┐
/// │   csr    │  rs1   │funct3│  rd  │ opcode │
/// │  12-bit  │ 5-bit  │3-bit │5-bit │ 7-bit  │
/// └──────────┴────────┴──────┴──────┴────────┘
///           SYSTEM (opcode = 0x73)
/// 
/// funct3:
///   001 = CSRRW
///   010 = CSRRS
///   011 = CSRRC
///   101 = CSRRWI
///   110 = CSRRSI
///   111 = CSRRCI
/// ```
pub static ZICSR_INSTRS: &[InstrDef] = &[
    // CSRRW: funct3 = 001
    InstrDef::new("CSRRW", I_TYPE_MASK, i_match(0b001, OP_SYSTEM), |raw| RvInstr::Csrrw {
        rd: rd(raw),
        rs1: rs1(raw),
        csr: csr_addr(raw),
    }),
    // CSRRS: funct3 = 010
    InstrDef::new("CSRRS", I_TYPE_MASK, i_match(0b010, OP_SYSTEM), |raw| RvInstr::Csrrs {
        rd: rd(raw),
        rs1: rs1(raw),
        csr: csr_addr(raw),
    }),
    // CSRRC: funct3 = 011
    InstrDef::new("CSRRC", I_TYPE_MASK, i_match(0b011, OP_SYSTEM), |raw| RvInstr::Csrrc {
        rd: rd(raw),
        rs1: rs1(raw),
        csr: csr_addr(raw),
    }),
    // CSRRWI: funct3 = 101
    InstrDef::new("CSRRWI", I_TYPE_MASK, i_match(0b101, OP_SYSTEM), |raw| RvInstr::Csrrwi {
        rd: rd(raw),
        zimm: rs1(raw),  // zimm 复用 rs1 字段
        csr: csr_addr(raw),
    }),
    // CSRRSI: funct3 = 110
    InstrDef::new("CSRRSI", I_TYPE_MASK, i_match(0b110, OP_SYSTEM), |raw| RvInstr::Csrrsi {
        rd: rd(raw),
        zimm: rs1(raw),
        csr: csr_addr(raw),
    }),
    // CSRRCI: funct3 = 111
    InstrDef::new("CSRRCI", I_TYPE_MASK, i_match(0b111, OP_SYSTEM), |raw| RvInstr::Csrrci {
        rd: rd(raw),
        zimm: rs1(raw),
        csr: csr_addr(raw),
    }),
];

/// Zicsr 扩展的 opcode 列表
pub static ZICSR_OPCODES: [u32; 1] = [OP_SYSTEM];

// ========== 解码器实例 ==========

/// Zicsr 解码器（基于 TableDrivenDecoder）
pub static ZICSR_DECODER: TableDrivenDecoder = TableDrivenDecoder::new(
    "Zicsr",
    ZICSR_INSTRS,
    Some(&ZICSR_OPCODES),
    true,
);

/// 兼容性别名
pub type ZicsrDecoder = TableDrivenDecoder;

// ========== CSR 地址常量 ==========

// Unprivileged CSRs
#[allow(dead_code)]
pub const CSR_CYCLE: u16 = 0xC00;
#[allow(dead_code)]
pub const CSR_TIME: u16 = 0xC01;
#[allow(dead_code)]
pub const CSR_INSTRET: u16 = 0xC02;
#[allow(dead_code)]
pub const CSR_CYCLEH: u16 = 0xC80;
#[allow(dead_code)]
pub const CSR_TIMEH: u16 = 0xC81;
#[allow(dead_code)]
pub const CSR_INSTRETH: u16 = 0xC82;

// Machine-level CSRs
#[allow(dead_code)]
pub const CSR_MVENDORID: u16 = 0xF11;
#[allow(dead_code)]
pub const CSR_MARCHID: u16 = 0xF12;
#[allow(dead_code)]
pub const CSR_MIMPID: u16 = 0xF13;
#[allow(dead_code)]
pub const CSR_MHARTID: u16 = 0xF14;
#[allow(dead_code)]
pub const CSR_MSTATUS: u16 = 0x300;
#[allow(dead_code)]
pub const CSR_MISA: u16 = 0x301;
#[allow(dead_code)]
pub const CSR_MEDELEG: u16 = 0x302;
#[allow(dead_code)]
pub const CSR_MIDELEG: u16 = 0x303;
#[allow(dead_code)]
pub const CSR_MIE: u16 = 0x304;
#[allow(dead_code)]
pub const CSR_MTVEC: u16 = 0x305;
#[allow(dead_code)]
pub const CSR_MCOUNTEREN: u16 = 0x306;
#[allow(dead_code)]
pub const CSR_MSCRATCH: u16 = 0x340;
#[allow(dead_code)]
pub const CSR_MEPC: u16 = 0x341;
#[allow(dead_code)]
pub const CSR_MCAUSE: u16 = 0x342;
#[allow(dead_code)]
pub const CSR_MTVAL: u16 = 0x343;
#[allow(dead_code)]
pub const CSR_MIP: u16 = 0x344;

// Supervisor-level CSRs
#[allow(dead_code)]
pub const CSR_SSTATUS: u16 = 0x100;
#[allow(dead_code)]
pub const CSR_SIE: u16 = 0x104;
#[allow(dead_code)]
pub const CSR_STVEC: u16 = 0x105;
#[allow(dead_code)]
pub const CSR_SCOUNTEREN: u16 = 0x106;
#[allow(dead_code)]
pub const CSR_SSCRATCH: u16 = 0x140;
#[allow(dead_code)]
pub const CSR_SEPC: u16 = 0x141;
#[allow(dead_code)]
pub const CSR_SCAUSE: u16 = 0x142;
#[allow(dead_code)]
pub const CSR_STVAL: u16 = 0x143;
#[allow(dead_code)]
pub const CSR_SIP: u16 = 0x144;
#[allow(dead_code)]
pub const CSR_SATP: u16 = 0x180;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::{DecodedInstr, InstrDecoder};

    #[test]
    fn test_decode_csrrw() {
        // csrrw x1, mstatus, x2
        // csr=0x300, rs1=x2, rd=x1, funct3=001, opcode=0x73
        // = 0x30011073 (but rd=1, so 0x300110F3)
        let raw = 0x300110F3;
        let decoded = ZICSR_DECODER.decode(raw);
        assert!(decoded.is_some());
        let DecodedInstr { instr, .. } = decoded.unwrap();
        match instr {
            RvInstr::Csrrw { rd, rs1, csr } => {
                assert_eq!(rd, 1);
                assert_eq!(rs1, 2);
                assert_eq!(csr, CSR_MSTATUS);
            }
            _ => panic!("Expected Csrrw"),
        }
    }

    #[test]
    fn test_decode_csrrs() {
        // csrrs x3, mie, x4 (读取 mie 并用 x4 置位)
        // csr=0x304, rs1=x4, rd=x3, funct3=010, opcode=0x73
        let raw = 0x304221F3;
        let decoded = ZICSR_DECODER.decode(raw);
        assert!(decoded.is_some());
        let DecodedInstr { instr, .. } = decoded.unwrap();
        match instr {
            RvInstr::Csrrs { rd, rs1, csr } => {
                assert_eq!(rd, 3);
                assert_eq!(rs1, 4);
                assert_eq!(csr, CSR_MIE);
            }
            _ => panic!("Expected Csrrs"),
        }
    }

    #[test]
    fn test_decode_csrrwi() {
        // csrrwi x5, mscratch, 7
        // csr=0x340, zimm=7, rd=x5, funct3=101, opcode=0x73
        let raw = 0x3403D2F3;
        let decoded = ZICSR_DECODER.decode(raw);
        assert!(decoded.is_some());
        let DecodedInstr { instr, .. } = decoded.unwrap();
        match instr {
            RvInstr::Csrrwi { rd, zimm, csr } => {
                assert_eq!(rd, 5);
                assert_eq!(zimm, 7);
                assert_eq!(csr, CSR_MSCRATCH);
            }
            _ => panic!("Expected Csrrwi"),
        }
    }

    #[test]
    fn test_decode_csrrc() {
        // csrrc x0, mstatus, x1 (清除 mstatus 中 x1 指定的位)
        let raw = 0x30013073;
        let decoded = ZICSR_DECODER.decode(raw);
        assert!(decoded.is_some());
        let DecodedInstr { instr, .. } = decoded.unwrap();
        match instr {
            RvInstr::Csrrc { rd, rs1, csr } => {
                assert_eq!(rd, 0);
                assert_eq!(rs1, 2);
                assert_eq!(csr, CSR_MSTATUS);
            }
            _ => panic!("Expected Csrrc"),
        }
    }
}
