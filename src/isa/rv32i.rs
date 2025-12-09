//! RV32I 基础指令集解码器
//!
//! 基于表驱动的解码实现

use crate::isa::fields::*;
use crate::isa::instr::RvInstr;
use crate::isa::instr_def::{
    InstrDef, TableDrivenDecoder,
    R_TYPE_MASK, I_TYPE_MASK, S_TYPE_MASK, B_TYPE_MASK,
    U_TYPE_MASK, J_TYPE_MASK, SHIFT_IMM_MASK, EXACT_MASK,
    r_match, i_match, shift_match,
};

// ========== RV32I 指令定义表 ==========

/// RV32I 指令定义表
pub static RV32I_INSTRS: &[InstrDef] = &[
    // ========== U-type ==========
    InstrDef::new("LUI", U_TYPE_MASK, OP_LUI, |raw| RvInstr::Lui {
        rd: rd(raw),
        imm: imm_u(raw),
    }),
    InstrDef::new("AUIPC", U_TYPE_MASK, OP_AUIPC, |raw| RvInstr::Auipc {
        rd: rd(raw),
        imm: imm_u(raw),
    }),
    
    // ========== J-type ==========
    InstrDef::new("JAL", J_TYPE_MASK, OP_JAL, |raw| RvInstr::Jal {
        rd: rd(raw),
        offset: imm_j(raw),
    }),
    
    // ========== I-type (JALR) ==========
    InstrDef::new("JALR", I_TYPE_MASK, i_match(0b000, OP_JALR), |raw| RvInstr::Jalr {
        rd: rd(raw),
        rs1: rs1(raw),
        offset: imm_i(raw),
    }),
    
    // ========== B-type ==========
    InstrDef::new("BEQ", B_TYPE_MASK, i_match(0b000, OP_BRANCH), |raw| RvInstr::Beq {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_b(raw),
    }),
    InstrDef::new("BNE", B_TYPE_MASK, i_match(0b001, OP_BRANCH), |raw| RvInstr::Bne {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_b(raw),
    }),
    InstrDef::new("BLT", B_TYPE_MASK, i_match(0b100, OP_BRANCH), |raw| RvInstr::Blt {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_b(raw),
    }),
    InstrDef::new("BGE", B_TYPE_MASK, i_match(0b101, OP_BRANCH), |raw| RvInstr::Bge {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_b(raw),
    }),
    InstrDef::new("BLTU", B_TYPE_MASK, i_match(0b110, OP_BRANCH), |raw| RvInstr::Bltu {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_b(raw),
    }),
    InstrDef::new("BGEU", B_TYPE_MASK, i_match(0b111, OP_BRANCH), |raw| RvInstr::Bgeu {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_b(raw),
    }),
    
    // ========== Load ==========
    InstrDef::new("LB", I_TYPE_MASK, i_match(0b000, OP_LOAD), |raw| RvInstr::Lb {
        rd: rd(raw),
        rs1: rs1(raw),
        offset: imm_i(raw),
    }),
    InstrDef::new("LH", I_TYPE_MASK, i_match(0b001, OP_LOAD), |raw| RvInstr::Lh {
        rd: rd(raw),
        rs1: rs1(raw),
        offset: imm_i(raw),
    }),
    InstrDef::new("LW", I_TYPE_MASK, i_match(0b010, OP_LOAD), |raw| RvInstr::Lw {
        rd: rd(raw),
        rs1: rs1(raw),
        offset: imm_i(raw),
    }),
    InstrDef::new("LBU", I_TYPE_MASK, i_match(0b100, OP_LOAD), |raw| RvInstr::Lbu {
        rd: rd(raw),
        rs1: rs1(raw),
        offset: imm_i(raw),
    }),
    InstrDef::new("LHU", I_TYPE_MASK, i_match(0b101, OP_LOAD), |raw| RvInstr::Lhu {
        rd: rd(raw),
        rs1: rs1(raw),
        offset: imm_i(raw),
    }),
    
    // ========== Store ==========
    InstrDef::new("SB", S_TYPE_MASK, i_match(0b000, OP_STORE), |raw| RvInstr::Sb {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_s(raw),
    }),
    InstrDef::new("SH", S_TYPE_MASK, i_match(0b001, OP_STORE), |raw| RvInstr::Sh {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_s(raw),
    }),
    InstrDef::new("SW", S_TYPE_MASK, i_match(0b010, OP_STORE), |raw| RvInstr::Sw {
        rs1: rs1(raw),
        rs2: rs2(raw),
        offset: imm_s(raw),
    }),
    
    // ========== I-type ALU ==========
    InstrDef::new("ADDI", I_TYPE_MASK, i_match(0b000, OP_IMM), |raw| RvInstr::Addi {
        rd: rd(raw),
        rs1: rs1(raw),
        imm: imm_i(raw),
    }),
    InstrDef::new("SLTI", I_TYPE_MASK, i_match(0b010, OP_IMM), |raw| RvInstr::Slti {
        rd: rd(raw),
        rs1: rs1(raw),
        imm: imm_i(raw),
    }),
    InstrDef::new("SLTIU", I_TYPE_MASK, i_match(0b011, OP_IMM), |raw| RvInstr::Sltiu {
        rd: rd(raw),
        rs1: rs1(raw),
        imm: imm_i(raw),
    }),
    InstrDef::new("XORI", I_TYPE_MASK, i_match(0b100, OP_IMM), |raw| RvInstr::Xori {
        rd: rd(raw),
        rs1: rs1(raw),
        imm: imm_i(raw),
    }),
    InstrDef::new("ORI", I_TYPE_MASK, i_match(0b110, OP_IMM), |raw| RvInstr::Ori {
        rd: rd(raw),
        rs1: rs1(raw),
        imm: imm_i(raw),
    }),
    InstrDef::new("ANDI", I_TYPE_MASK, i_match(0b111, OP_IMM), |raw| RvInstr::Andi {
        rd: rd(raw),
        rs1: rs1(raw),
        imm: imm_i(raw),
    }),
    
    // ========== Shift immediate ==========
    InstrDef::new("SLLI", SHIFT_IMM_MASK, shift_match(0b000000, 0b001, OP_IMM), |raw| RvInstr::Slli {
        rd: rd(raw),
        rs1: rs1(raw),
        shamt: shamt(raw),
    }),
    InstrDef::new("SRLI", SHIFT_IMM_MASK, shift_match(0b000000, 0b101, OP_IMM), |raw| RvInstr::Srli {
        rd: rd(raw),
        rs1: rs1(raw),
        shamt: shamt(raw),
    }),
    InstrDef::new("SRAI", SHIFT_IMM_MASK, shift_match(0b010000, 0b101, OP_IMM), |raw| RvInstr::Srai {
        rd: rd(raw),
        rs1: rs1(raw),
        shamt: shamt(raw),
    }),
    
    // ========== R-type ==========
    InstrDef::new("ADD", R_TYPE_MASK, r_match(0b0000000, 0b000, OP_REG), |raw| RvInstr::Add {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("SUB", R_TYPE_MASK, r_match(0b0100000, 0b000, OP_REG), |raw| RvInstr::Sub {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("SLL", R_TYPE_MASK, r_match(0b0000000, 0b001, OP_REG), |raw| RvInstr::Sll {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("SLT", R_TYPE_MASK, r_match(0b0000000, 0b010, OP_REG), |raw| RvInstr::Slt {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("SLTU", R_TYPE_MASK, r_match(0b0000000, 0b011, OP_REG), |raw| RvInstr::Sltu {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("XOR", R_TYPE_MASK, r_match(0b0000000, 0b100, OP_REG), |raw| RvInstr::Xor {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("SRL", R_TYPE_MASK, r_match(0b0000000, 0b101, OP_REG), |raw| RvInstr::Srl {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("SRA", R_TYPE_MASK, r_match(0b0100000, 0b101, OP_REG), |raw| RvInstr::Sra {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("OR", R_TYPE_MASK, r_match(0b0000000, 0b110, OP_REG), |raw| RvInstr::Or {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    InstrDef::new("AND", R_TYPE_MASK, r_match(0b0000000, 0b111, OP_REG), |raw| RvInstr::And {
        rd: rd(raw),
        rs1: rs1(raw),
        rs2: rs2(raw),
    }),
    
    // ========== Fence & 系统 ==========
    InstrDef::new("FENCE", I_TYPE_MASK, i_match(0b000, OP_MISC_MEM), |raw| {
        let imm = ((raw >> 20) & 0x0FFF) as u16;
        let pred = ((imm >> 4) & 0xF) as u8;
        let succ = (imm & 0xF) as u8;
        let fm = ((imm >> 8) & 0xF) as u8;
        RvInstr::Fence { pred, succ, fm }
    }),
    InstrDef::new("FENCE.I", I_TYPE_MASK, i_match(0b001, OP_MISC_MEM), |_| RvInstr::FenceI),

    // ========== 系统 ==========
    InstrDef::new("ECALL", EXACT_MASK, 0x00000073, |_| RvInstr::Ecall),
    InstrDef::new("EBREAK", EXACT_MASK, 0x00100073, |_| RvInstr::Ebreak),
];

/// RV32I 基础指令集的 opcode 列表
pub static RV32I_OPCODES: [u32; 11] = [
    OP_LUI, OP_AUIPC, OP_JAL, OP_JALR, OP_BRANCH,
    OP_LOAD, OP_STORE, OP_MISC_MEM, OP_IMM, OP_REG, OP_SYSTEM,
];

// ========== 解码器实例 ==========

/// RV32I 解码器（基于 TableDrivenDecoder）
pub static RV32I_DECODER: TableDrivenDecoder = TableDrivenDecoder::new(
    "RV32I",
    RV32I_INSTRS,
    Some(&RV32I_OPCODES),
    true,
);

/// 兼容性别名
pub type Rv32iDecoder = TableDrivenDecoder;
