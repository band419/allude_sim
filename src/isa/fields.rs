//! 指令字段提取辅助函数
//!
//! 提供从 32-bit 指令字中提取各字段的工具函数

/// 提取 opcode 字段 [6:0]
#[inline]
pub fn opcode(raw: u32) -> u32 {
    raw & 0x7F
}

/// 提取 rd 字段 [11:7]
#[inline]
pub fn rd(raw: u32) -> u8 {
    ((raw >> 7) & 0x1F) as u8
}

/// 提取 funct3 字段 [14:12]
#[inline]
pub fn funct3(raw: u32) -> u32 {
    (raw >> 12) & 0x7
}

/// 提取 rs1 字段 [19:15]
#[inline]
pub fn rs1(raw: u32) -> u8 {
    ((raw >> 15) & 0x1F) as u8
}

/// 提取 rs2 字段 [24:20]
#[inline]
pub fn rs2(raw: u32) -> u8 {
    ((raw >> 20) & 0x1F) as u8
}

/// 提取 funct7 字段 [31:25]
#[inline]
pub fn funct7(raw: u32) -> u32 {
    (raw >> 25) & 0x7F
}

/// 提取 I-type 立即数并符号扩展
/// imm[11:0] = raw[31:20]
#[inline]
pub fn imm_i(raw: u32) -> i32 {
    (raw as i32) >> 20
}

/// 提取 S-type 立即数并符号扩展
/// imm[11:5] = raw[31:25], imm[4:0] = raw[11:7]
#[inline]
pub fn imm_s(raw: u32) -> i32 {
    let imm_11_5 = (raw >> 25) & 0x7F;
    let imm_4_0 = (raw >> 7) & 0x1F;
    let imm = (imm_11_5 << 5) | imm_4_0;
    ((imm as i32) << 20) >> 20
}

/// 提取 B-type 立即数并符号扩展
/// imm[12] = raw[31], imm[10:5] = raw[30:25], imm[4:1] = raw[11:8], imm[11] = raw[7]
#[inline]
pub fn imm_b(raw: u32) -> i32 {
    let imm_12 = (raw >> 31) & 0x1;
    let imm_10_5 = (raw >> 25) & 0x3F;
    let imm_4_1 = (raw >> 8) & 0xF;
    let imm_11 = (raw >> 7) & 0x1;
    let imm = (imm_12 << 12) | (imm_11 << 11) | (imm_10_5 << 5) | (imm_4_1 << 1);
    ((imm as i32) << 19) >> 19
}

/// 提取 U-type 立即数
/// imm[31:12] = raw[31:12], imm[11:0] = 0
#[inline]
pub fn imm_u(raw: u32) -> i32 {
    (raw & 0xFFFFF000) as i32
}

/// 提取 J-type 立即数并符号扩展
/// imm[20] = raw[31], imm[10:1] = raw[30:21], imm[11] = raw[20], imm[19:12] = raw[19:12]
#[inline]
pub fn imm_j(raw: u32) -> i32 {
    let imm_20 = (raw >> 31) & 0x1;
    let imm_10_1 = (raw >> 21) & 0x3FF;
    let imm_11 = (raw >> 20) & 0x1;
    let imm_19_12 = (raw >> 12) & 0xFF;
    let imm = (imm_20 << 20) | (imm_19_12 << 12) | (imm_11 << 11) | (imm_10_1 << 1);
    ((imm as i32) << 11) >> 11
}

/// 提取移位量 shamt [24:20]
#[inline]
pub fn shamt(raw: u32) -> u8 {
    ((raw >> 20) & 0x1F) as u8
}

/// 提取 CSR 地址 [31:20]
#[inline]
pub fn csr_addr(raw: u32) -> u16 {
    ((raw >> 20) & 0xFFF) as u16
}

/// 提取 CSR 立即数 (zimm) [19:15]，零扩展的 5-bit 无符号数
#[inline]
pub fn csr_zimm(raw: u32) -> u32 {
    (raw >> 15) & 0x1F
}

// ========== Opcode 常量 ==========
pub const OP_LUI: u32 = 0b0110111;
pub const OP_AUIPC: u32 = 0b0010111;
pub const OP_JAL: u32 = 0b1101111;
pub const OP_JALR: u32 = 0b1100111;
pub const OP_BRANCH: u32 = 0b1100011;
pub const OP_LOAD: u32 = 0b0000011;
pub const OP_STORE: u32 = 0b0100011;
pub const OP_MISC_MEM: u32 = 0b0001111;
pub const OP_IMM: u32 = 0b0010011;
pub const OP_REG: u32 = 0b0110011;
pub const OP_SYSTEM: u32 = 0b1110011;

// RISC-V 预留的自定义 opcode 空间
pub const OP_CUSTOM_0: u32 = 0b0001011;
pub const OP_CUSTOM_1: u32 = 0b0101011;
pub const OP_CUSTOM_2: u32 = 0b1011011;
pub const OP_CUSTOM_3: u32 = 0b1111011;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imm_b_positive() {
        let raw = 0x00208463; // beq x1, x2, 8
        assert_eq!(imm_b(raw), 8);
    }

    #[test]
    fn test_imm_j_positive() {
        let raw = 0x0040006F; // jal x0, 4
        assert_eq!(imm_j(raw), 4);
    }

    #[test]
    fn test_imm_s() {
        let raw = 0x00112423; // sw x1, 8(x2)
        assert_eq!(imm_s(raw), 8);
    }
}
