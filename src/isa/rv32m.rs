//! RV32M 扩展（乘除法）解码器
//!
//! 基于表驱动的解码实现

use crate::isa::fields::*;
use crate::isa::instr::RvInstr;
use crate::isa::instr_def::{InstrDef, TableDrivenDecoder, R_TYPE_MASK, r_match};

// ========== RV32M 指令定义表 ==========

/// RV32M 指令定义表
pub static RV32M_INSTRS: &[InstrDef] = &[
    InstrDef::new("MUL", R_TYPE_MASK, r_match(0b0000001, 0b000, OP_REG), |raw| RvInstr::Mul {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("MULH", R_TYPE_MASK, r_match(0b0000001, 0b001, OP_REG), |raw| RvInstr::Mulh {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("MULHSU", R_TYPE_MASK, r_match(0b0000001, 0b010, OP_REG), |raw| RvInstr::Mulhsu {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("MULHU", R_TYPE_MASK, r_match(0b0000001, 0b011, OP_REG), |raw| RvInstr::Mulhu {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("DIV", R_TYPE_MASK, r_match(0b0000001, 0b100, OP_REG), |raw| RvInstr::Div {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("DIVU", R_TYPE_MASK, r_match(0b0000001, 0b101, OP_REG), |raw| RvInstr::Divu {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("REM", R_TYPE_MASK, r_match(0b0000001, 0b110, OP_REG), |raw| RvInstr::Rem {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("REMU", R_TYPE_MASK, r_match(0b0000001, 0b111, OP_REG), |raw| RvInstr::Remu {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
];

/// RV32M 扩展的 opcode 列表
pub static RV32M_OPCODES: [u32; 1] = [OP_REG];

// ========== 解码器实例 ==========

/// RV32M 解码器（基于 TableDrivenDecoder）
/// 
/// M 扩展在 OP (0b0110011) opcode 空间内，通过 funct7 = 0b0000001 区分
pub static RV32M_DECODER: TableDrivenDecoder = TableDrivenDecoder::new(
    "RV32M",
    RV32M_INSTRS,
    Some(&RV32M_OPCODES),
    true,
);

/// 兼容性别名
pub type Rv32mDecoder = TableDrivenDecoder;
