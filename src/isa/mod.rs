//! RISC-V ISA 抽象与解码框架
//!
//! 本模块提供可扩展的指令解码系统：
//! - `RvInstr`: 指令的语义表示（enum + 自定义扩展）
//! - `InstrDecoder`: 解码器 trait，允许插件式扩展
//! - `DecoderRegistry`: 解码器注册表，支持运行时注册
//! - `InstrDef`: 统一的指令定义，同时用于解码和冲突检测
//! - `IsaConfig`: ISA 配置构建器，支持冲突检测

mod decoder;
mod instr;
mod fields;
mod instr_def;
mod rv32i;
mod rv32m;
mod rv32f;
mod zicsr;
mod config;
mod priv_instr;

pub use decoder::{InstrDecoder, DecoderRegistry};
pub use instr::{RvInstr, DecodedInstr, CustomInstr, CustomFields};
pub use fields::*;
pub use instr_def::{InstrDef, TableDrivenDecoder};
pub use rv32i::{RV32I_DECODER, RV32I_INSTRS, RV32I_OPCODES, Rv32iDecoder};
pub use rv32m::{RV32M_DECODER, RV32M_INSTRS, RV32M_OPCODES, Rv32mDecoder};
pub use rv32f::{RV32F_DECODER, RV32F_INSTRS, RV32F_OPCODES, Rv32fDecoder, RoundingMode};
pub use zicsr::{ZICSR_DECODER, ZICSR_INSTRS, ZICSR_OPCODES, ZicsrDecoder};
pub use priv_instr::{PRIV_DECODER, PRIV_INSTRS, PRIV_OPCODES, MRET_ENCODING, SRET_ENCODING, WFI_ENCODING};
pub use config::{IsaConfig, IsaExtension, ConflictInfo};

/// 便捷函数：使用默认 RV32I 解码器解码指令
/// 
/// 这保持了与旧 API 的兼容性
pub fn decode(raw: u32) -> DecodedInstr {
    RV32I_DECODER.decode(raw).unwrap_or(DecodedInstr {
        raw,
        instr: RvInstr::Illegal { raw },
    })
}

#[cfg(test)]
mod tests;
