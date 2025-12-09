//! RV32F 扩展（单精度浮点）解码器
//!
//! 实现 RISC-V F 标准扩展的指令解码

use crate::isa::fields::*;
use crate::isa::instr::RvInstr;
use crate::isa::instr_def::{InstrDef, TableDrivenDecoder};

// ========== F 扩展 opcode ==========

/// LOAD-FP opcode (FLW)
pub const OP_LOAD_FP: u32 = 0b0000111;
/// STORE-FP opcode (FSW)
pub const OP_STORE_FP: u32 = 0b0100111;
/// MADD opcode (FMADD.S)
pub const OP_MADD: u32 = 0b1000011;
/// MSUB opcode (FMSUB.S)
pub const OP_MSUB: u32 = 0b1000111;
/// NMSUB opcode (FNMSUB.S)
pub const OP_NMSUB: u32 = 0b1001011;
/// NMADD opcode (FNMADD.S)
pub const OP_NMADD: u32 = 0b1001111;
/// OP-FP opcode (浮点运算)
pub const OP_FP: u32 = 0b1010011;

// ========== R4-type 指令掩码 ==========

/// R4-type 指令掩码 (用于 FMADD 等)
/// 检查 opcode[6:0], fmt[26:25]
pub const R4_TYPE_MASK: u32 = 0x0600007F;

/// R4-type 匹配值构造
#[inline]
pub const fn r4_match(fmt: u32, opcode: u32) -> u32 {
    (fmt << 25) | opcode
}

// ========== FP R-type 指令掩码 ==========

/// FP R-type 指令掩码
/// 检查 opcode[6:0], funct7[31:25]
pub const FP_R_TYPE_MASK: u32 = 0xFE00007F;

/// FP R-type 匹配值构造
#[inline]
pub const fn fp_r_match(funct7: u32, opcode: u32) -> u32 {
    (funct7 << 25) | opcode
}

// ========== funct7 编码 ==========

pub const FADD_S: u32 = 0b0000000;
pub const FSUB_S: u32 = 0b0000100;
pub const FMUL_S: u32 = 0b0001000;
pub const FDIV_S: u32 = 0b0001100;
pub const FSQRT_S: u32 = 0b0101100;
pub const FSGNJ_S: u32 = 0b0010000;  // funct3 区分 FSGNJ/FSGNJN/FSGNJX
pub const FMINMAX_S: u32 = 0b0010100; // funct3 区分 FMIN/FMAX
pub const FCVT_W_S: u32 = 0b1100000;  // rs2 区分 FCVT.W.S / FCVT.WU.S
pub const FMV_X_W: u32 = 0b1110000;   // 也包括 FCLASS.S
pub const FCMP_S: u32 = 0b1010000;    // funct3 区分 FEQ/FLT/FLE
pub const FCVT_S_W: u32 = 0b1101000;  // rs2 区分 FCVT.S.W / FCVT.S.WU
pub const FMV_W_X: u32 = 0b1111000;

// ========== 字段提取函数 ==========

/// 提取 rs3 字段 [31:27] (用于 R4-type 指令)
#[inline]
pub fn rs3(raw: u32) -> u8 {
    ((raw >> 27) & 0x1F) as u8
}

/// 提取舍入模式 rm [14:12]
#[inline]
pub fn rm(raw: u32) -> u8 {
    ((raw >> 12) & 0x7) as u8
}

/// 提取格式 fmt [26:25]
#[inline]
#[allow(dead_code)]
pub fn fmt(raw: u32) -> u8 {
    ((raw >> 25) & 0x3) as u8
}

// ========== 舍入模式常量 ==========

/// 舍入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RoundingMode {
    /// 向最近偶数舍入
    Rne = 0b000,
    /// 向零舍入
    Rtz = 0b001,
    /// 向负无穷舍入
    Rdn = 0b010,
    /// 向正无穷舍入
    Rup = 0b011,
    /// 向最近舍入，远离零
    Rmm = 0b100,
    /// 使用 frm CSR 中的舍入模式
    Dyn = 0b111,
}

impl From<u8> for RoundingMode {
    fn from(val: u8) -> Self {
        match val {
            0b000 => RoundingMode::Rne,
            0b001 => RoundingMode::Rtz,
            0b010 => RoundingMode::Rdn,
            0b011 => RoundingMode::Rup,
            0b100 => RoundingMode::Rmm,
            _ => RoundingMode::Dyn,
        }
    }
}

// ========== RV32F 指令定义表 ==========

/// RV32F 指令定义表
pub static RV32F_INSTRS: &[InstrDef] = &[
    // ========== 加载/存储 ==========
    // FLW: frd = M[rs1 + imm]
    InstrDef::new("FLW", 0x0000707F, (0b010 << 12) | OP_LOAD_FP, |raw| RvInstr::Flw {
        frd: rd(raw),
        rs1: rs1(raw),
        offset: imm_i(raw),
    }),
    // FSW: M[rs1 + imm] = frs2
    InstrDef::new("FSW", 0x0000707F, (0b010 << 12) | OP_STORE_FP, |raw| RvInstr::Fsw {
        frs2: rs2(raw),
        rs1: rs1(raw),
        offset: imm_s(raw),
    }),

    // ========== 融合乘加 (R4-type) ==========
    // FMADD.S: frd = frs1 * frs2 + frs3
    InstrDef::new("FMADD.S", R4_TYPE_MASK, r4_match(0b00, OP_MADD), |raw| RvInstr::FmaddS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        frs3: rs3(raw),
        rm: rm(raw),
    }),
    // FMSUB.S: frd = frs1 * frs2 - frs3
    InstrDef::new("FMSUB.S", R4_TYPE_MASK, r4_match(0b00, OP_MSUB), |raw| RvInstr::FmsubS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        frs3: rs3(raw),
        rm: rm(raw),
    }),
    // FNMSUB.S: frd = -frs1 * frs2 + frs3
    InstrDef::new("FNMSUB.S", R4_TYPE_MASK, r4_match(0b00, OP_NMSUB), |raw| RvInstr::FnmsubS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        frs3: rs3(raw),
        rm: rm(raw),
    }),
    // FNMADD.S: frd = -frs1 * frs2 - frs3
    InstrDef::new("FNMADD.S", R4_TYPE_MASK, r4_match(0b00, OP_NMADD), |raw| RvInstr::FnmaddS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        frs3: rs3(raw),
        rm: rm(raw),
    }),

    // ========== 算术运算 ==========
    // FADD.S
    InstrDef::new("FADD.S", FP_R_TYPE_MASK, fp_r_match(FADD_S, OP_FP), |raw| RvInstr::FaddS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        rm: rm(raw),
    }),
    // FSUB.S
    InstrDef::new("FSUB.S", FP_R_TYPE_MASK, fp_r_match(FSUB_S, OP_FP), |raw| RvInstr::FsubS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        rm: rm(raw),
    }),
    // FMUL.S
    InstrDef::new("FMUL.S", FP_R_TYPE_MASK, fp_r_match(FMUL_S, OP_FP), |raw| RvInstr::FmulS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        rm: rm(raw),
    }),
    // FDIV.S
    InstrDef::new("FDIV.S", FP_R_TYPE_MASK, fp_r_match(FDIV_S, OP_FP), |raw| RvInstr::FdivS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
        rm: rm(raw),
    }),
    // FSQRT.S (rs2 must be 0)
    InstrDef::new("FSQRT.S", 0xFFF0007F, fp_r_match(FSQRT_S, OP_FP), |raw| RvInstr::FsqrtS {
        frd: rd(raw),
        frs1: rs1(raw),
        rm: rm(raw),
    }),

    // ========== 符号注入 ==========
    // FSGNJ.S (funct3 = 000)
    InstrDef::new("FSGNJ.S", 0xFE00707F, fp_r_match(FSGNJ_S, OP_FP) | (0b000 << 12), |raw| RvInstr::FsgnjS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),
    // FSGNJN.S (funct3 = 001)
    InstrDef::new("FSGNJN.S", 0xFE00707F, fp_r_match(FSGNJ_S, OP_FP) | (0b001 << 12), |raw| RvInstr::FsgnjnS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),
    // FSGNJX.S (funct3 = 010)
    InstrDef::new("FSGNJX.S", 0xFE00707F, fp_r_match(FSGNJ_S, OP_FP) | (0b010 << 12), |raw| RvInstr::FsgnjxS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),

    // ========== 最小/最大 ==========
    // FMIN.S (funct3 = 000)
    InstrDef::new("FMIN.S", 0xFE00707F, fp_r_match(FMINMAX_S, OP_FP) | (0b000 << 12), |raw| RvInstr::FminS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),
    // FMAX.S (funct3 = 001)
    InstrDef::new("FMAX.S", 0xFE00707F, fp_r_match(FMINMAX_S, OP_FP) | (0b001 << 12), |raw| RvInstr::FmaxS {
        frd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),

    // ========== 比较 ==========
    // FEQ.S (funct3 = 010)
    InstrDef::new("FEQ.S", 0xFE00707F, fp_r_match(FCMP_S, OP_FP) | (0b010 << 12), |raw| RvInstr::FeqS {
        rd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),
    // FLT.S (funct3 = 001)
    InstrDef::new("FLT.S", 0xFE00707F, fp_r_match(FCMP_S, OP_FP) | (0b001 << 12), |raw| RvInstr::FltS {
        rd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),
    // FLE.S (funct3 = 000)
    InstrDef::new("FLE.S", 0xFE00707F, fp_r_match(FCMP_S, OP_FP) | (0b000 << 12), |raw| RvInstr::FleS {
        rd: rd(raw),
        frs1: rs1(raw),
        frs2: rs2(raw),
    }),

    // ========== 类型转换 ==========
    // FCVT.W.S (rs2 = 0)
    InstrDef::new("FCVT.W.S", 0xFFF0007F, fp_r_match(FCVT_W_S, OP_FP), |raw| RvInstr::FcvtWS {
        rd: rd(raw),
        frs1: rs1(raw),
        rm: rm(raw),
    }),
    // FCVT.WU.S (rs2 = 1)
    InstrDef::new("FCVT.WU.S", 0xFFF0007F, fp_r_match(FCVT_W_S, OP_FP) | (1 << 20), |raw| RvInstr::FcvtWuS {
        rd: rd(raw),
        frs1: rs1(raw),
        rm: rm(raw),
    }),
    // FCVT.S.W (rs2 = 0)
    InstrDef::new("FCVT.S.W", 0xFFF0007F, fp_r_match(FCVT_S_W, OP_FP), |raw| RvInstr::FcvtSW {
        frd: rd(raw),
        rs1: rs1(raw),
        rm: rm(raw),
    }),
    // FCVT.S.WU (rs2 = 1)
    InstrDef::new("FCVT.S.WU", 0xFFF0007F, fp_r_match(FCVT_S_W, OP_FP) | (1 << 20), |raw| RvInstr::FcvtSWu {
        frd: rd(raw),
        rs1: rs1(raw),
        rm: rm(raw),
    }),

    // ========== 移动指令 ==========
    // FMV.X.W (funct3 = 000, rs2 = 0)
    InstrDef::new("FMV.X.W", 0xFFF0707F, fp_r_match(FMV_X_W, OP_FP) | (0b000 << 12), |raw| RvInstr::FmvXW {
        rd: rd(raw),
        frs1: rs1(raw),
    }),
    // FCLASS.S (funct3 = 001, rs2 = 0)
    InstrDef::new("FCLASS.S", 0xFFF0707F, fp_r_match(FMV_X_W, OP_FP) | (0b001 << 12), |raw| RvInstr::FclassS {
        rd: rd(raw),
        frs1: rs1(raw),
    }),
    // FMV.W.X (funct3 = 000, rs2 = 0)
    InstrDef::new("FMV.W.X", 0xFFF0707F, fp_r_match(FMV_W_X, OP_FP) | (0b000 << 12), |raw| RvInstr::FmvWX {
        frd: rd(raw),
        rs1: rs1(raw),
    }),
];

/// RV32F 使用的 opcode 列表
pub static RV32F_OPCODES: [u32; 7] = [
    OP_LOAD_FP,
    OP_STORE_FP,
    OP_MADD,
    OP_MSUB,
    OP_NMSUB,
    OP_NMADD,
    OP_FP,
];

// ========== 解码器实例 ==========

/// RV32F 解码器
pub static RV32F_DECODER: TableDrivenDecoder = TableDrivenDecoder::new(
    "RV32F",
    RV32F_INSTRS,
    Some(&RV32F_OPCODES),
    false,
);

/// 兼容性别名
pub type Rv32fDecoder = TableDrivenDecoder;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::InstrDecoder;

    #[test]
    fn test_decode_flw() {
        // flw f1, 4(x2) = imm=4, rs1=2, funct3=010, rd=1, opcode=0000111
        // 0000 0000 0100 00010 010 00001 0000111 = 0x00412087
        let raw = 0x00412087;
        let decoded = RV32F_DECODER.decode(raw);
        assert!(decoded.is_some());
        match decoded.unwrap().instr {
            RvInstr::Flw { frd, rs1, offset } => {
                assert_eq!(frd, 1);
                assert_eq!(rs1, 2);
                assert_eq!(offset, 4);
            }
            _ => panic!("Expected Flw"),
        }
    }

    #[test]
    fn test_decode_fsw() {
        // fsw f1, 8(x2) = imm[11:5]=0, rs2=1, rs1=2, funct3=010, imm[4:0]=8, opcode=0100111
        // 0000 000 00001 00010 010 01000 0100111 = 0x00112427
        let raw = 0x00112427;
        let decoded = RV32F_DECODER.decode(raw);
        assert!(decoded.is_some());
        match decoded.unwrap().instr {
            RvInstr::Fsw { frs2, rs1, offset } => {
                assert_eq!(frs2, 1);
                assert_eq!(rs1, 2);
                assert_eq!(offset, 8);
            }
            _ => panic!("Expected Fsw"),
        }
    }

    #[test]
    fn test_decode_fadd_s() {
        // fadd.s f1, f2, f3, rne = funct7=0000000, rs2=3, rs1=2, rm=000, rd=1, opcode=1010011
        // 0000000 00011 00010 000 00001 1010011 = 0x003100D3
        let raw = 0x003100D3;
        let decoded = RV32F_DECODER.decode(raw);
        assert!(decoded.is_some());
        match decoded.unwrap().instr {
            RvInstr::FaddS { frd, frs1, frs2, rm } => {
                assert_eq!(frd, 1);
                assert_eq!(frs1, 2);
                assert_eq!(frs2, 3);
                assert_eq!(rm, 0);
            }
            _ => panic!("Expected FaddS"),
        }
    }

    #[test]
    fn test_decode_fmadd_s() {
        // fmadd.s f1, f2, f3, f4, rne
        // rs3=4, fmt=00, rs2=3, rs1=2, rm=000, rd=1, opcode=1000011
        // 00100 00 00011 00010 000 00001 1000011 = 0x203100C3
        let raw = 0x203100C3;
        let decoded = RV32F_DECODER.decode(raw);
        assert!(decoded.is_some());
        match decoded.unwrap().instr {
            RvInstr::FmaddS { frd, frs1, frs2, frs3, rm } => {
                assert_eq!(frd, 1);
                assert_eq!(frs1, 2);
                assert_eq!(frs2, 3);
                assert_eq!(frs3, 4);
                assert_eq!(rm, 0);
            }
            _ => panic!("Expected FmaddS"),
        }
    }

    #[test]
    fn test_decode_fcvt_w_s() {
        // fcvt.w.s x1, f2, rtz
        // funct7=1100000, rs2=0, rs1=2, rm=001, rd=1, opcode=1010011
        // 1100000 00000 00010 001 00001 1010011 = 0xC00110D3
        let raw = 0xC00110D3;
        let decoded = RV32F_DECODER.decode(raw);
        assert!(decoded.is_some());
        match decoded.unwrap().instr {
            RvInstr::FcvtWS { rd, frs1, rm } => {
                assert_eq!(rd, 1);
                assert_eq!(frs1, 2);
                assert_eq!(rm, 1);
            }
            _ => panic!("Expected FcvtWS"),
        }
    }

    #[test]
    fn test_decode_fmv_x_w() {
        // fmv.x.w x1, f2
        // funct7=1110000, rs2=0, rs1=2, funct3=000, rd=1, opcode=1010011
        // 1110000 00000 00010 000 00001 1010011 = 0xE00100D3
        let raw = 0xE00100D3;
        let decoded = RV32F_DECODER.decode(raw);
        assert!(decoded.is_some());
        match decoded.unwrap().instr {
            RvInstr::FmvXW { rd, frs1 } => {
                assert_eq!(rd, 1);
                assert_eq!(frs1, 2);
            }
            _ => panic!("Expected FmvXW"),
        }
    }
}
