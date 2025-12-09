//! 指令定义结构
//!
//! 统一的指令定义，同时用于解码和冲突检测

use super::decoder::InstrDecoder;
use super::instr::{DecodedInstr, RvInstr};

/// 指令定义
/// 
/// 一处定义，两处使用：
/// - 解码：通过 mask/match 匹配后调用 decode 函数
/// - 冲突检测：通过 mask/match 判断两条指令是否可能冲突
#[derive(Clone)]
pub struct InstrDef {
    /// 指令名称（用于调试和冲突报告）
    pub name: &'static str,
    /// 匹配掩码：哪些位需要检查
    pub mask: u32,
    /// 匹配值：这些位应该是什么
    pub match_val: u32,
    /// 解码函数：从原始编码提取字段并构造 RvInstr
    pub decode: fn(u32) -> RvInstr,
}

impl InstrDef {
    /// 创建新的指令定义
    pub const fn new(
        name: &'static str,
        mask: u32,
        match_val: u32,
        decode: fn(u32) -> RvInstr,
    ) -> Self {
        Self {
            name,
            mask,
            match_val,
            decode,
        }
    }

    /// 检查指令是否匹配此定义
    #[inline]
    pub fn matches(&self, raw: u32) -> bool {
        (raw & self.mask) == self.match_val
    }

    /// 解码指令
    #[inline]
    pub fn decode_instr(&self, raw: u32) -> DecodedInstr {
        DecodedInstr {
            raw,
            instr: (self.decode)(raw),
        }
    }

    /// 检查两个指令定义是否冲突
    /// 
    /// 两个定义冲突当且仅当存在某个指令字同时匹配两者
    pub fn conflicts_with(&self, other: &InstrDef) -> bool {
        let common_mask = self.mask & other.mask;
        (self.match_val & common_mask) == (other.match_val & common_mask)
    }
}

impl std::fmt::Debug for InstrDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstrDef")
            .field("name", &self.name)
            .field("mask", &format_args!("0x{:08X}", self.mask))
            .field("match_val", &format_args!("0x{:08X}", self.match_val))
            .finish()
    }
}

// ========== 类型掩码常量 ==========

/// R-type 指令的 mask（检查 opcode + funct3 + funct7）
pub const R_TYPE_MASK: u32 = 0xFE00707F;

/// I-type 指令的 mask（检查 opcode + funct3）
pub const I_TYPE_MASK: u32 = 0x707F;

/// S-type 指令的 mask（检查 opcode + funct3）
pub const S_TYPE_MASK: u32 = 0x707F;

/// B-type 指令的 mask（检查 opcode + funct3）
pub const B_TYPE_MASK: u32 = 0x707F;

/// U-type 指令的 mask（只检查 opcode）
pub const U_TYPE_MASK: u32 = 0x7F;

/// J-type 指令的 mask（只检查 opcode）
pub const J_TYPE_MASK: u32 = 0x7F;

/// Shift-imm 指令的 mask（检查 opcode + funct3 + funct7 高位）
pub const SHIFT_IMM_MASK: u32 = 0xFC00707F;

/// 精确匹配整个指令（用于 ECALL/EBREAK）
pub const EXACT_MASK: u32 = 0xFFFFFFFF;

// ========== 辅助函数：构造 match 值 ==========

/// 构造 R-type 的 match 值
#[inline]
pub const fn r_match(funct7: u32, funct3: u32, opcode: u32) -> u32 {
    (funct7 << 25) | (funct3 << 12) | opcode
}

/// 构造 I-type 的 match 值
#[inline]
pub const fn i_match(funct3: u32, opcode: u32) -> u32 {
    (funct3 << 12) | opcode
}

/// 构造 shift-imm 的 match 值（带 funct7 高 6 位）
#[inline]
pub const fn shift_match(funct6: u32, funct3: u32, opcode: u32) -> u32 {
    (funct6 << 26) | (funct3 << 12) | opcode
}

// ========== 表驱动解码器 ==========

/// 表驱动解码器
/// 
/// 通用解码器，使用 InstrDef 数组进行解码
#[derive(Clone, Copy)]
pub struct TableDrivenDecoder {
    /// 解码器名称
    name: &'static str,
    /// 指令定义表
    instrs: &'static [InstrDef],
    /// 处理的 opcode（用于优化）
    opcodes: Option<&'static [u32]>,
    /// 是否允许 opcode 共享（如 RV32I/RV32M 共用 0x33）
    allow_overlap: bool,
}

impl TableDrivenDecoder {
    /// 创建新的表驱动解码器
    pub const fn new(
        name: &'static str,
        instrs: &'static [InstrDef],
        opcodes: Option<&'static [u32]>,
        allow_overlap: bool,
    ) -> Self {
        Self { name, instrs, opcodes, allow_overlap }
    }

    /// 获取指令定义表
    pub fn instrs(&self) -> &'static [InstrDef] {
        self.instrs
    }
}

impl InstrDecoder for TableDrivenDecoder {
    fn name(&self) -> &str {
        self.name
    }

    fn decode(&self, raw: u32) -> Option<DecodedInstr> {
        for def in self.instrs {
            if def.matches(raw) {
                return Some(def.decode_instr(raw));
            }
        }
        None
    }

    fn handled_opcodes(&self) -> Option<&[u32]> {
        self.opcodes
    }

    fn allow_opcode_overlap(&self) -> bool {
        self.allow_overlap
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::rv32i::RV32I_INSTRS;
    use crate::isa::rv32m::RV32M_INSTRS;

    #[test]
    fn test_instr_def_matches() {
        let add_def = RV32I_INSTRS.iter().find(|d| d.name == "ADD").unwrap();
        
        // add x3, x1, x2: funct7=0, rs2=2, rs1=1, funct3=0, rd=3, op=0110011
        let add_raw = 0x002081B3;
        assert!(add_def.matches(add_raw));
        
        // sub x3, x1, x2: funct7=0100000
        let sub_raw = 0x402081B3;
        assert!(!add_def.matches(sub_raw));
    }

    #[test]
    fn test_instr_def_decode() {
        let addi_def = RV32I_INSTRS.iter().find(|d| d.name == "ADDI").unwrap();
        
        let raw = 0x02A00093; // addi x1, x0, 42
        let decoded = addi_def.decode_instr(raw);
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
    fn test_no_conflict_add_mul() {
        let add_def = RV32I_INSTRS.iter().find(|d| d.name == "ADD").unwrap();
        let mul_def = RV32M_INSTRS.iter().find(|d| d.name == "MUL").unwrap();
        
        // ADD: funct7=0000000, MUL: funct7=0000001
        assert!(!add_def.conflicts_with(mul_def));
    }

    #[test]
    fn test_conflict_detection() {
        // 创建两个冲突的定义
        let def1 = InstrDef::new("TEST1", I_TYPE_MASK, 0x0033, |_| RvInstr::Ecall);
        let def2 = InstrDef::new("TEST2", I_TYPE_MASK, 0x0033, |_| RvInstr::Ebreak);
        
        assert!(def1.conflicts_with(&def2));
    }

    #[test]
    fn test_rv32i_coverage() {
        // 确保 RV32I 定义表完整
        assert!(RV32I_INSTRS.len() >= 37, "RV32I 应该有至少 37 条指令");
    }

    #[test]
    fn test_rv32m_coverage() {
        assert_eq!(RV32M_INSTRS.len(), 8, "RV32M 应该有 8 条指令");
    }
}
