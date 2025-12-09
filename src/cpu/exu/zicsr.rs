//! Zicsr 扩展执行单元
//!
//! 实现 CSR 操作指令的执行逻辑

use super::super::CpuCore;
use crate::isa::RvInstr;

/// 执行 Zicsr 指令。返回 true 如果处理了该指令。
pub fn execute(cpu: &mut CpuCore, instr: RvInstr) -> bool {
    match instr {
        // CSRRW: t = CSR[csr]; CSR[csr] = rs1; rd = t
        // 特例：当 rd = x0 时，不读取 CSR（可能有副作用的 CSR 不会被读取）
        RvInstr::Csrrw { rd, rs1, csr } => {
            let rs1_val = cpu.read_reg(rs1);
            if rd != 0 {
                let old_val = cpu.csr_read(csr);
                cpu.write_reg(rd, old_val);
            }
            cpu.csr_write(csr, rs1_val);
        }
        
        // CSRRS: t = CSR[csr]; CSR[csr] = t | rs1; rd = t
        // 特例：当 rs1 = x0 时，不写入 CSR（纯读取操作）
        RvInstr::Csrrs { rd, rs1, csr } => {
            let old_val = cpu.csr_read(csr);
            cpu.write_reg(rd, old_val);
            if rs1 != 0 {
                let rs1_val = cpu.read_reg(rs1);
                cpu.csr_write(csr, old_val | rs1_val);
            }
        }
        
        // CSRRC: t = CSR[csr]; CSR[csr] = t & ~rs1; rd = t
        // 特例：当 rs1 = x0 时，不写入 CSR（纯读取操作）
        RvInstr::Csrrc { rd, rs1, csr } => {
            let old_val = cpu.csr_read(csr);
            cpu.write_reg(rd, old_val);
            if rs1 != 0 {
                let rs1_val = cpu.read_reg(rs1);
                cpu.csr_write(csr, old_val & !rs1_val);
            }
        }
        
        // CSRRWI: t = CSR[csr]; CSR[csr] = zimm; rd = t
        // 特例：当 rd = x0 时，不读取 CSR
        RvInstr::Csrrwi { rd, zimm, csr } => {
            if rd != 0 {
                let old_val = cpu.csr_read(csr);
                cpu.write_reg(rd, old_val);
            }
            cpu.csr_write(csr, zimm as u32);
        }
        
        // CSRRSI: t = CSR[csr]; CSR[csr] = t | zimm; rd = t
        // 特例：当 zimm = 0 时，不写入 CSR（纯读取操作）
        RvInstr::Csrrsi { rd, zimm, csr } => {
            let old_val = cpu.csr_read(csr);
            cpu.write_reg(rd, old_val);
            if zimm != 0 {
                cpu.csr_write(csr, old_val | (zimm as u32));
            }
        }
        
        // CSRRCI: t = CSR[csr]; CSR[csr] = t & ~zimm; rd = t
        // 特例：当 zimm = 0 时，不写入 CSR（纯读取操作）
        RvInstr::Csrrci { rd, zimm, csr } => {
            let old_val = cpu.csr_read(csr);
            cpu.write_reg(rd, old_val);
            if zimm != 0 {
                cpu.csr_write(csr, old_val & !(zimm as u32));
            }
        }
        
        _ => return false,
    }
    
    true
}
