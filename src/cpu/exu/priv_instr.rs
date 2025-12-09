//! 特权指令执行单元
//!
//! 实现 MRET、SRET、WFI 等特权指令的执行逻辑

use super::super::CpuCore;
use super::super::csr_def::{CSR_MEPC, CSR_MSTATUS, CSR_SEPC, CSR_SSTATUS};
use super::super::trap::{mstatus, PrivilegeMode};
use super::super::CpuState;
use crate::isa::RvInstr;

/// 执行特权指令。返回 true 如果处理了该指令。
pub fn execute(cpu: &mut CpuCore, instr: RvInstr) -> bool {
    match instr {
        RvInstr::Mret => {
            execute_mret(cpu);
            true
        }
        RvInstr::Sret => {
            execute_sret(cpu);
            true
        }
        RvInstr::Wfi => {
            execute_wfi(cpu);
            true
        }
        _ => false,
    }
}

/// 执行 MRET 指令：从 M-mode trap handler 返回
///
/// 执行流程：
/// 1. 将 mstatus.MPIE 恢复到 mstatus.MIE
/// 2. 将特权级设置为 mstatus.MPP
/// 3. 将 mstatus.MPP 设置为 U (或 M，如果不支持 U)
/// 4. 将 mstatus.MPIE 设置为 1
/// 5. PC = mepc
fn execute_mret(cpu: &mut CpuCore) {
    let mstatus_val = cpu.csr_read(CSR_MSTATUS);
    
    // 读取保存的状态
    let mpie = mstatus::read_mpie(mstatus_val);
    let mpp = mstatus::read_mpp(mstatus_val);
    
    // 更新 mstatus
    let mut new_mstatus = mstatus_val;
    
    // MIE = MPIE
    if mpie {
        new_mstatus |= mstatus::MIE_MASK;
    } else {
        new_mstatus &= !mstatus::MIE_MASK;
    }
    
    // MPP = U (0) 或 M (3)，取决于是否支持 U-mode
    // 目前简化处理：设置为 U-mode
    new_mstatus = mstatus::write_mpp(new_mstatus, 0);
    
    // MPIE = 1
    new_mstatus |= mstatus::MPIE_MASK;
    
    cpu.csr_write(CSR_MSTATUS, new_mstatus);
    
    // 设置特权级为 MPP
    cpu.set_privilege(PrivilegeMode::from_bits(mpp));
    
    // PC = mepc
    let mepc = cpu.csr_read(CSR_MEPC);
    cpu.set_pc(mepc);
}

/// 执行 SRET 指令：从 S-mode trap handler 返回
///
/// 类似 MRET，但操作 sstatus.SPIE/SPP 和 sepc
fn execute_sret(cpu: &mut CpuCore) {
    let sstatus_val = cpu.csr_read(CSR_SSTATUS);
    
    // 读取保存的状态 (SPP 是 1 位，位置 8)
    let spie = (sstatus_val >> mstatus::SPIE) & 1 != 0;
    let spp = ((sstatus_val >> mstatus::SPP) & 1) as u8;
    
    // 更新 sstatus
    let mut new_sstatus = sstatus_val;
    
    // SIE = SPIE
    if spie {
        new_sstatus |= mstatus::SIE_MASK;
    } else {
        new_sstatus &= !mstatus::SIE_MASK;
    }
    
    // SPP = U (0)
    new_sstatus &= !mstatus::SPP_MASK;
    
    // SPIE = 1
    new_sstatus |= mstatus::SPIE_MASK;
    
    cpu.csr_write(CSR_SSTATUS, new_sstatus);
    
    // 设置特权级为 SPP (0=U, 1=S)
    let new_mode = if spp == 0 {
        PrivilegeMode::User
    } else {
        PrivilegeMode::Supervisor
    };
    cpu.set_privilege(new_mode);
    
    // PC = sepc
    let sepc = cpu.csr_read(CSR_SEPC);
    cpu.set_pc(sepc);
}

/// 执行 WFI 指令：等待中断
///
/// 暂停执行直到有中断发生
fn execute_wfi(cpu: &mut CpuCore) {
    cpu.set_state(CpuState::WaitForInterrupt);
}
