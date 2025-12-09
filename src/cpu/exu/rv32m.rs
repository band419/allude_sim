use super::super::CpuCore;
use crate::isa::RvInstr;

/// Execute RV32M (mul/div) instructions. Returns true if handled.
pub fn execute(cpu: &mut CpuCore, instr: RvInstr) -> bool {
    match instr {
        RvInstr::Mul { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1);
            let b = cpu.read_reg(rs2);
            let result = a.wrapping_mul(b);
            cpu.write_reg(rd, result);
        }
        RvInstr::Mulh { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1) as i32 as i64;
            let b = cpu.read_reg(rs2) as i32 as i64;
            let result = ((a * b) >> 32) as u32;
            cpu.write_reg(rd, result);
        }
        RvInstr::Mulhsu { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1) as i32 as i64;
            let b = cpu.read_reg(rs2) as u64 as i64;
            let result = ((a * b) >> 32) as u32;
            cpu.write_reg(rd, result);
        }
        RvInstr::Mulhu { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1) as u64;
            let b = cpu.read_reg(rs2) as u64;
            let result = ((a * b) >> 32) as u32;
            cpu.write_reg(rd, result);
        }
        RvInstr::Div { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1) as i32;
            let b = cpu.read_reg(rs2) as i32;
            let result = if b == 0 {
                -1i32 as u32
            } else if a == i32::MIN && b == -1 {
                a as u32
            } else {
                (a / b) as u32
            };
            cpu.write_reg(rd, result);
        }
        RvInstr::Divu { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1);
            let b = cpu.read_reg(rs2);
            let result = if b == 0 { u32::MAX } else { a / b };
            cpu.write_reg(rd, result);
        }
        RvInstr::Rem { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1) as i32;
            let b = cpu.read_reg(rs2) as i32;
            let result = if b == 0 {
                a as u32
            } else if a == i32::MIN && b == -1 {
                0
            } else {
                (a % b) as u32
            };
            cpu.write_reg(rd, result);
        }
        RvInstr::Remu { rd, rs1, rs2 } => {
            let a = cpu.read_reg(rs1);
            let b = cpu.read_reg(rs2);
            let result = if b == 0 { a } else { a % b };
            cpu.write_reg(rd, result);
        }
        _ => return false,
    }

    true
}
