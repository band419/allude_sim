//! 特权指令解码器
//!
//! 定义 MRET、SRET、WFI 等特权指令

use crate::isa::instr::RvInstr;
use crate::isa::instr_def::{InstrDef, TableDrivenDecoder, EXACT_MASK};
use crate::isa::fields::OP_SYSTEM;

// ========== 特权指令编码 ==========

/// MRET 指令编码: 0011000 00010 00000 000 00000 1110011
/// = 0x30200073
pub const MRET_ENCODING: u32 = 0x30200073;

/// SRET 指令编码: 0001000 00010 00000 000 00000 1110011
/// = 0x10200073
pub const SRET_ENCODING: u32 = 0x10200073;

/// WFI 指令编码: 0001000 00101 00000 000 00000 1110011
/// = 0x10500073
pub const WFI_ENCODING: u32 = 0x10500073;

// ========== 特权指令定义表 ==========

/// 特权指令定义表
pub static PRIV_INSTRS: &[InstrDef] = &[
    InstrDef::new("MRET", EXACT_MASK, MRET_ENCODING, |_| RvInstr::Mret),
    InstrDef::new("SRET", EXACT_MASK, SRET_ENCODING, |_| RvInstr::Sret),
    InstrDef::new("WFI", EXACT_MASK, WFI_ENCODING, |_| RvInstr::Wfi),
];

/// 特权指令使用的 opcode
pub static PRIV_OPCODES: [u32; 1] = [OP_SYSTEM];

// ========== 解码器实例 ==========

/// 特权指令解码器
/// 
/// 注意：allow_overlap 设为 true，因为 SYSTEM opcode (0x73) 已被 RV32I 的
/// ECALL/EBREAK 使用，特权指令需要与其共存
pub static PRIV_DECODER: TableDrivenDecoder = TableDrivenDecoder::new(
    "Priv",
    PRIV_INSTRS,
    Some(&PRIV_OPCODES),
    true, // 允许与 RV32I 共享 SYSTEM opcode
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::InstrDecoder;

    #[test]
    fn test_decode_mret() {
        let instr = PRIV_DECODER.decode(MRET_ENCODING);
        assert!(instr.is_some());
        assert_eq!(instr.unwrap().instr, RvInstr::Mret);
    }

    #[test]
    fn test_decode_sret() {
        let instr = PRIV_DECODER.decode(SRET_ENCODING);
        assert!(instr.is_some());
        assert_eq!(instr.unwrap().instr, RvInstr::Sret);
    }

    #[test]
    fn test_decode_wfi() {
        let instr = PRIV_DECODER.decode(WFI_ENCODING);
        assert!(instr.is_some());
        assert_eq!(instr.unwrap().instr, RvInstr::Wfi);
    }
}
