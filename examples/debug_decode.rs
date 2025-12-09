// 调试脚本：验证 B 型指令编码

use allude_sim::isa::{decode, RvInstr};

fn main() {
    // 测试 beq x0, x0, -16 的编码
    // B-type 立即数格式：imm[12|10:5] rs2 rs1 funct3 imm[4:1|11] opcode
    // offset = -16 = 0xFFFF_FFF0 (32-bit)
    // 13 位表示: 1_1111_1111_0000 (imm[12:1], imm[0] 总是 0)
    // imm[12]=1, imm[11]=1, imm[10:5]=111110, imm[4:1]=0000
    //
    // 编码: imm[12] | imm[10:5] | rs2 | rs1 | funct3 | imm[4:1] | imm[11] | opcode
    //       1       | 111110    | 00000| 00000| 000   | 0000     | 1       | 1100011
    // = 1_111110_00000_00000_000_0000_1_1100011
    // = FE000CE3
    
    println!("=== B-type 指令编码测试 ===\n");
    
    // 测试 beq x0, x0, -16
    let test_cases: &[(u32, &str, i32)] = &[
        (0xFE0008E3, "beq x0, x0, -16", -16),
        (0xFE000CE3, "beq x0, x0, -8", -8),
        (0x00208463, "beq x1, x2, 8", 8),
        (0x0041D663, "bge x3, x4, 12", 12),
    ];
    
    for (raw, desc, expected_offset) in test_cases {
        let decoded = decode(*raw);
        println!("指令: {} (0x{:08X})", desc, raw);
        println!("  解码结果: {:?}", decoded.instr);
        
        match decoded.instr {
            RvInstr::Beq { rs1: _, rs2: _, offset } => {
                println!("  偏移量: {} (预期: {})", offset, expected_offset);
                if offset == *expected_offset {
                    println!("  ✓ 正确");
                } else {
                    println!("  ✗ 错误!");
                }
            }
            RvInstr::Bge { rs1: _, rs2: _, offset } => {
                println!("  偏移量: {} (预期: {})", offset, expected_offset);
                if offset == *expected_offset {
                    println!("  ✓ 正确");
                } else {
                    println!("  ✗ 错误!");
                }
            }
            _ => println!("  ✗ 解码类型错误!"),
        }
        println!();
    }
    
    // 手动计算 beq x0, x0, -16 的正确编码
    println!("=== 手动编码计算 ===\n");
    
    // beq x0, x0, offset=-16
    // offset = -16: 二进制 (13位有符号) = 1_1111_1111_0000
    // imm[12]=1, imm[11]=1, imm[10:5]=111110, imm[4:1]=0000
    let offset = -16i32;
    let imm_12 = ((offset >> 12) & 1) as u32;
    let imm_11 = ((offset >> 11) & 1) as u32;
    let imm_10_5 = ((offset >> 5) & 0x3F) as u32;
    let imm_4_1 = ((offset >> 1) & 0xF) as u32;
    
    let rs1 = 0u32; // x0
    let rs2 = 0u32; // x0
    let funct3 = 0u32; // beq
    let opcode = 0b1100011u32;
    
    let encoded = (imm_12 << 31) | (imm_10_5 << 25) | (rs2 << 20) | (rs1 << 15) 
                | (funct3 << 12) | (imm_4_1 << 8) | (imm_11 << 7) | opcode;
    
    println!("beq x0, x0, -16 应该编码为: 0x{:08X}", encoded);
    
    // 验证解码
    let decoded = decode(encoded);
    println!("解码结果: {:?}", decoded.instr);
}
