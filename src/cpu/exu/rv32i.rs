use super::super::{CpuCore, MemAccessType};
use super::super::trap::TrapCause;
use crate::isa::RvInstr;
use crate::memory::Memory;

/// Execute RV32I-base instructions. Returns true if handled.
pub fn execute(cpu: &mut CpuCore, mem: &mut dyn Memory, instr: RvInstr, current_pc: u32) -> bool {
    match instr {
        // ========== R-type 算术/逻辑指令 ==========
        RvInstr::Add { rd, rs1, rs2 } => {
            let result = cpu.read_reg(rs1).wrapping_add(cpu.read_reg(rs2));
            cpu.write_reg(rd, result);
        }
        RvInstr::Sub { rd, rs1, rs2 } => {
            let result = cpu.read_reg(rs1).wrapping_sub(cpu.read_reg(rs2));
            cpu.write_reg(rd, result);
        }
        RvInstr::And { rd, rs1, rs2 } => {
            let result = cpu.read_reg(rs1) & cpu.read_reg(rs2);
            cpu.write_reg(rd, result);
        }
        RvInstr::Or { rd, rs1, rs2 } => {
            let result = cpu.read_reg(rs1) | cpu.read_reg(rs2);
            cpu.write_reg(rd, result);
        }
        RvInstr::Xor { rd, rs1, rs2 } => {
            let result = cpu.read_reg(rs1) ^ cpu.read_reg(rs2);
            cpu.write_reg(rd, result);
        }
        RvInstr::Slt { rd, rs1, rs2 } => {
            let result = if (cpu.read_reg(rs1) as i32) < (cpu.read_reg(rs2) as i32) {
                1
            } else {
                0
            };
            cpu.write_reg(rd, result);
        }
        RvInstr::Sltu { rd, rs1, rs2 } => {
            let result = if cpu.read_reg(rs1) < cpu.read_reg(rs2) { 1 } else { 0 };
            cpu.write_reg(rd, result);
        }
        RvInstr::Sll { rd, rs1, rs2 } => {
            let shamt = cpu.read_reg(rs2) & 0x1F;
            let result = cpu.read_reg(rs1) << shamt;
            cpu.write_reg(rd, result);
        }
        RvInstr::Srl { rd, rs1, rs2 } => {
            let shamt = cpu.read_reg(rs2) & 0x1F;
            let result = cpu.read_reg(rs1) >> shamt;
            cpu.write_reg(rd, result);
        }
        RvInstr::Sra { rd, rs1, rs2 } => {
            let shamt = cpu.read_reg(rs2) & 0x1F;
            let result = ((cpu.read_reg(rs1) as i32) >> shamt) as u32;
            cpu.write_reg(rd, result);
        }

        // ========== I-type 立即数算术/逻辑指令 ==========
        RvInstr::Addi { rd, rs1, imm } => {
            let result = cpu.read_reg(rs1).wrapping_add(imm as u32);
            cpu.write_reg(rd, result);
        }
        RvInstr::Andi { rd, rs1, imm } => {
            let result = cpu.read_reg(rs1) & (imm as u32);
            cpu.write_reg(rd, result);
        }
        RvInstr::Ori { rd, rs1, imm } => {
            let result = cpu.read_reg(rs1) | (imm as u32);
            cpu.write_reg(rd, result);
        }
        RvInstr::Xori { rd, rs1, imm } => {
            let result = cpu.read_reg(rs1) ^ (imm as u32);
            cpu.write_reg(rd, result);
        }
        RvInstr::Slti { rd, rs1, imm } => {
            let result = if (cpu.read_reg(rs1) as i32) < imm { 1 } else { 0 };
            cpu.write_reg(rd, result);
        }
        RvInstr::Sltiu { rd, rs1, imm } => {
            let result = if cpu.read_reg(rs1) < (imm as u32) { 1 } else { 0 };
            cpu.write_reg(rd, result);
        }
        RvInstr::Slli { rd, rs1, shamt } => {
            let result = cpu.read_reg(rs1) << shamt;
            cpu.write_reg(rd, result);
        }
        RvInstr::Srli { rd, rs1, shamt } => {
            let result = cpu.read_reg(rs1) >> shamt;
            cpu.write_reg(rd, result);
        }
        RvInstr::Srai { rd, rs1, shamt } => {
            let result = ((cpu.read_reg(rs1) as i32) >> shamt) as u32;
            cpu.write_reg(rd, result);
        }

        // ========== Load 指令 ==========
        RvInstr::Lb { rd, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = match cpu.mem_result(mem.load8(addr), MemAccessType::Load, current_pc) {
                Some(v) => v as i8 as i32 as u32,
                None => return true,
            };
            cpu.write_reg(rd, value);
        }
        RvInstr::Lh { rd, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = match load_halfword(cpu, mem, addr, true, current_pc) {
                Some(v) => v,
                None => return true,
            };
            cpu.write_reg(rd, value);
        }
        RvInstr::Lw { rd, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = match load_word(cpu, mem, addr, current_pc) {
                Some(v) => v,
                None => return true,
            };
            cpu.write_reg(rd, value);
        }
        RvInstr::Lbu { rd, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = match cpu.mem_result(mem.load8(addr), MemAccessType::Load, current_pc) {
                Some(v) => v as u32,
                None => return true,
            };
            cpu.write_reg(rd, value);
        }
        RvInstr::Lhu { rd, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = match load_halfword(cpu, mem, addr, false, current_pc) {
                Some(v) => v,
                None => return true,
            };
            cpu.write_reg(rd, value);
        }

        // ========== Store 指令 ==========
        RvInstr::Sb { rs1, rs2, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = cpu.read_reg(rs2) as u8;
            if !cpu.mem_result_unit(mem.store8(addr, value), MemAccessType::Store, current_pc) {
                return true;
            }
        }
        RvInstr::Sh { rs1, rs2, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = cpu.read_reg(rs2) as u16;
            if !store_halfword(cpu, mem, addr, value, current_pc) {
                return true;
            }
        }
        RvInstr::Sw { rs1, rs2, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = cpu.read_reg(rs2);
            if !store_word(cpu, mem, addr, value, current_pc) {
                return true;
            }
        }

        // ========== U-type 指令 ==========
        RvInstr::Lui { rd, imm } => {
            cpu.write_reg(rd, imm as u32);
        }
        RvInstr::Auipc { rd, imm } => {
            let result = current_pc.wrapping_add(imm as u32);
            cpu.write_reg(rd, result);
        }

        // ========== 控制流指令 ==========
        RvInstr::Jal { rd, offset } => {
            cpu.write_reg(rd, cpu.pc());
            cpu.set_pc(current_pc.wrapping_add(offset as u32));
        }
        RvInstr::Jalr { rd, rs1, offset } => {
            let return_addr = cpu.pc();
            let target = cpu.read_reg(rs1).wrapping_add(offset as u32) & !1;
            cpu.write_reg(rd, return_addr);
            cpu.set_pc(target);
        }
        RvInstr::Beq { rs1, rs2, offset } => {
            if cpu.read_reg(rs1) == cpu.read_reg(rs2) {
                cpu.set_pc(current_pc.wrapping_add(offset as u32));
            }
        }
        RvInstr::Bne { rs1, rs2, offset } => {
            if cpu.read_reg(rs1) != cpu.read_reg(rs2) {
                cpu.set_pc(current_pc.wrapping_add(offset as u32));
            }
        }
        RvInstr::Blt { rs1, rs2, offset } => {
            if (cpu.read_reg(rs1) as i32) < (cpu.read_reg(rs2) as i32) {
                cpu.set_pc(current_pc.wrapping_add(offset as u32));
            }
        }
        RvInstr::Bge { rs1, rs2, offset } => {
            if (cpu.read_reg(rs1) as i32) >= (cpu.read_reg(rs2) as i32) {
                cpu.set_pc(current_pc.wrapping_add(offset as u32));
            }
        }
        RvInstr::Bltu { rs1, rs2, offset } => {
            if cpu.read_reg(rs1) < cpu.read_reg(rs2) {
                cpu.set_pc(current_pc.wrapping_add(offset as u32));
            }
        }
        RvInstr::Bgeu { rs1, rs2, offset } => {
            if cpu.read_reg(rs1) >= cpu.read_reg(rs2) {
                cpu.set_pc(current_pc.wrapping_add(offset as u32));
            }
        }

        // ========== 系统指令 ==========
        RvInstr::Ecall => {
            // 根据当前特权级生成对应的 ecall 异常
            let cause = TrapCause::ecall_from(cpu.privilege());
            // 注意：current_pc 是触发异常的指令地址
            cpu.take_trap_at(cause, 0, current_pc);
        }
        RvInstr::Ebreak => {
            // 触发断点异常
            cpu.take_trap_at(TrapCause::Breakpoint, current_pc, current_pc);
        }
        RvInstr::Fence { pred, succ, fm } => {
            let _ = (pred, succ, fm); // 单核模型中视为立即完成
        }
        RvInstr::FenceI => {
            // 简化实现：不模拟指令缓存，视为 NOP
        }

        _ => return false,
    }

    true
}

fn load_halfword(
    cpu: &mut CpuCore,
    mem: &mut dyn Memory,
    addr: u32,
    signed: bool,
    current_pc: u32,
) -> Option<u32> {
    if addr & 0x1 == 0 {
        let raw = cpu.mem_result(mem.load16(addr), MemAccessType::Load, current_pc)?;
        return Some(if signed { raw as i16 as i32 as u32 } else { raw as u32 });
    }

    let b0 = cpu.mem_result(mem.load8(addr), MemAccessType::Load, current_pc)?;
    let b1 = cpu.mem_result(mem.load8(addr.wrapping_add(1)), MemAccessType::Load, current_pc)?;
    let raw = u16::from_le_bytes([b0, b1]);
    Some(if signed { raw as i16 as i32 as u32 } else { raw as u32 })
}

fn load_word(
    cpu: &mut CpuCore,
    mem: &mut dyn Memory,
    addr: u32,
    current_pc: u32,
) -> Option<u32> {
    if addr & 0x3 == 0 {
        return cpu.mem_result(mem.load32(addr), MemAccessType::Load, current_pc);
    }

    let mut bytes = [0u8; 4];
    for i in 0..4 {
        bytes[i] = cpu.mem_result(
            mem.load8(addr.wrapping_add(i as u32)),
            MemAccessType::Load,
            current_pc,
        )?;
    }
    Some(u32::from_le_bytes(bytes))
}

fn store_halfword(
    cpu: &mut CpuCore,
    mem: &mut dyn Memory,
    addr: u32,
    value: u16,
    current_pc: u32,
) -> bool {
    if addr & 0x1 == 0 {
        return cpu.mem_result_unit(mem.store16(addr, value), MemAccessType::Store, current_pc);
    }

    let bytes = value.to_le_bytes();
    if !cpu.mem_result_unit(mem.store8(addr, bytes[0]), MemAccessType::Store, current_pc) {
        return false;
    }
    if !cpu.mem_result_unit(
        mem.store8(addr.wrapping_add(1), bytes[1]),
        MemAccessType::Store,
        current_pc,
    ) {
        return false;
    }
    true
}

fn store_word(
    cpu: &mut CpuCore,
    mem: &mut dyn Memory,
    addr: u32,
    value: u32,
    current_pc: u32,
) -> bool {
    if addr & 0x3 == 0 {
        return cpu.mem_result_unit(mem.store32(addr, value), MemAccessType::Store, current_pc);
    }

    let bytes = value.to_le_bytes();
    for i in 0..4 {
        if !cpu.mem_result_unit(
            mem.store8(addr.wrapping_add(i as u32), bytes[i]),
            MemAccessType::Store,
            current_pc,
        ) {
            return false;
        }
    }
    true
}
