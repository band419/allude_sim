//! CPU 核心与执行引擎
//!
//! 本模块定义了单线程 RV32I CPU 核心 `CpuCore`，
//! 包含寄存器文件、程序计数器以及执行引擎。

use std::sync::Arc;

use crate::isa::{self, DecodedInstr, RvInstr, DecoderRegistry};
use crate::memory::Memory;

mod exu;
pub mod csr_def;
mod status;
mod builder;
pub mod trap;

use status::Status;
pub use status::{CsrEntry, StatusSnapshot};
pub use builder::CpuBuilder;
pub use trap::{TrapCause, PrivilegeMode};

/// CPU 执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    /// 正常运行中
    Running,
    /// 遇到非法指令
    IllegalInstruction(u32),
    /// 等待中断 (WFI 指令)
    WaitForInterrupt,
    /// CPU 已停机
    Halted,
}

/// 单线程 CPU 核心
///
/// 包含 RV32I 的最小状态：
/// - 32 个 32-bit 通用寄存器 x0..x31（x0 恒为 0）
/// - 32-bit 程序计数器
///
/// 设计约定：
/// - x0 永远为 0，写入时丢弃
/// - PC 为字节地址，所有指令 4 字节对齐
/// - 核心状态不依赖全局变量，方便多线程/warp 结构重用
pub struct CpuCore {
    /// 架构状态（寄存器文件 + CSR）
    status: Status,
    /// 程序计数器
    pc: u32,
    /// 当前 CPU 状态
    state: CpuState,
    /// 指令解码器
    decoder: Arc<DecoderRegistry>,
}

impl CpuCore {
    /// 创建一个新的 CPU 核心
    ///
    /// # 参数
    ///
    /// * `entry_pc` - 初始程序计数器值
    ///
    /// # 示例
    ///
    /// ```
    /// use allude_sim::cpu::CpuCore;
    ///
    /// let cpu = CpuCore::new(0x1000);
    /// assert_eq!(cpu.pc(), 0x1000);
    /// ```
    pub fn new(entry_pc: u32) -> Self {
        // 默认使用 RV32I 解码器
        let decoder = Arc::new(isa::IsaConfig::new().build().expect("RV32I should not conflict"));
        CpuCore {
            status: Status::new(),
            pc: entry_pc,
            state: CpuState::Running,
            decoder,
        }
    }

    /// 使用预配置的状态和解码器创建 CPU 核心
    pub(crate) fn with_config(entry_pc: u32, status: Status, decoder: Arc<DecoderRegistry>) -> Self {
        CpuCore {
            status,
            pc: entry_pc,
            state: CpuState::Running,
            decoder,
        }
    }

    /// 获取当前程序计数器值
    pub fn pc(&self) -> u32 {
        self.pc
    }

    /// 设置程序计数器
    pub fn set_pc(&mut self, pc: u32) {
        self.pc = pc;
    }

    /// 获取当前 CPU 状态
    pub fn state(&self) -> CpuState {
        self.state
    }

    /// 读取 x0 总是返回 0
    pub fn read_reg(&self, reg: u8) -> u32 {
        self.status.int_read(reg)
    }

  
    pub fn write_reg(&mut self, reg: u8, value: u32) {
        self.status.int_write(reg, value)
    }

    pub fn read_fp(&self, reg: u8) -> u32 {
        self.status.fp.as_ref().map(|fp| fp.read(reg)).unwrap_or(0)
    }

    /// 如果 F 扩展未启用，写入会被忽略
    pub fn write_fp(&mut self, reg: u8, value: u32) {
        if let Some(fp) = self.status.fp.as_mut() {
            fp.write(reg, value);
        }
    }

    pub fn read_fp_f32(&self, reg: u8) -> f32 {
        f32::from_bits(self.read_fp(reg))
    }

    pub fn write_fp_f32(&mut self, reg: u8, value: f32) {
        self.write_fp(reg, value.to_bits());
    }

    /// 检查是否启用了浮点扩展
    pub fn has_fp(&self) -> bool {
        self.status.fp.is_some()
    }

    /// CSR 值，如果未注册则返回 0
    pub fn csr_read(&self, csr: u16) -> u32 {
        self.status.csr_read(csr)
    }

    pub fn csr_write(&mut self, csr: u16, value: u32) {
        self.status.csr_write(csr, value)
    }

   
    pub fn privilege(&self) -> PrivilegeMode {
        self.status.privilege
    }

    pub fn set_privilege(&mut self, mode: PrivilegeMode) {
        self.status.privilege = mode;
    }

    /// 设置 CPU 状态
    pub fn set_state(&mut self, state: CpuState) {
        self.state = state;
    }

    /// 触发 trap（异常或中断）
    ///
    /// 执行 RISC-V 特权规范定义的 trap 处理流程：
    /// 1. 保存当前 PC 到 mepc/sepc
    /// 2. 保存异常原因到 mcause/scause
    /// 3. 保存额外信息到 mtval/stval
    /// 4. 更新 mstatus/sstatus（保存中断使能位等）
    /// 5. 设置新特权级
    /// 6. 跳转到 trap handler (mtvec/stvec)
    ///
    /// # 参数
    ///
    /// * `cause` - trap 原因
    /// * `tval` - 额外信息（如错误地址、非法指令编码等）
    pub fn take_trap(&mut self, cause: TrapCause, tval: u32) {
        // 使用当前 PC 作为异常 PC
        self.take_trap_at(cause, tval, self.pc);
    }

    /// 在指定 PC 处触发 trap（用于异常需要指向触发指令的情况）
    ///
    /// # 参数
    ///
    /// * `cause` - trap 原因
    /// * `tval` - 额外信息（如错误地址、非法指令编码等）
    /// * `epc` - 异常 PC（保存到 mepc）
    pub fn take_trap_at(&mut self, cause: TrapCause, tval: u32, epc: u32) {
        use csr_def::*;
        use trap::{mstatus, calculate_trap_pc};

        // 目前简化实现：所有 trap 都进入 M-mode
        // TODO: 支持 trap 委托 (medeleg/mideleg)
        let target_mode = PrivilegeMode::Machine;

        // 保存异常 PC 到 mepc
        // 对于异常：mepc 指向触发异常的指令
        // 对于中断：mepc 指向下一条要执行的指令
        self.status.csr_write(CSR_MEPC, epc);

        // 保存异常原因到 mcause
        self.status.csr_write(CSR_MCAUSE, cause.to_cause_value());

        // 保存额外信息到 mtval
        self.status.csr_write(CSR_MTVAL, tval);

        // 更新 mstatus
        let mstatus = self.status.csr_read(CSR_MSTATUS);
        
        // 保存当前 MIE 到 MPIE
        let mie = mstatus::read_mie(mstatus);
        let mut new_mstatus = mstatus;
        
        // MPIE = MIE
        if mie {
            new_mstatus |= mstatus::MPIE_MASK;
        } else {
            new_mstatus &= !mstatus::MPIE_MASK;
        }
        
        // MIE = 0 (禁用中断)
        new_mstatus &= !mstatus::MIE_MASK;
        
        // MPP = current privilege
        new_mstatus = mstatus::write_mpp(new_mstatus, self.status.privilege.to_bits());
        
        self.status.csr_write(CSR_MSTATUS, new_mstatus);

        // 设置新特权级
        self.status.privilege = target_mode;

        // 跳转到 trap handler
        let mtvec = self.status.csr_read(CSR_MTVEC);
        self.pc = calculate_trap_pc(mtvec, &cause);
    }

    /// 获取所有寄存器的快照
    pub fn regs(&self) -> &[u32; 32] {
        self.status.int_snapshot()
    }

    /// 获取完整架构状态快照
    pub fn snapshot(&self) -> StatusSnapshot {
        self.status.snapshot()
    }

    /// 执行单步指令
    ///
    /// # 参数
    ///
    /// * `mem` - 内存接口
    ///
    /// # 返回
    ///
    /// 当前 CPU 状态
    ///
    /// # 流程
    ///
    /// 1. 从 PC 处取指
    /// 2. 解码指令
    /// 3. 默认 PC += 4
    /// 4. 执行指令（可能修改 PC）
    pub fn step(&mut self, mem: &mut dyn Memory) -> CpuState {
        if self.state != CpuState::Running {
            return self.state;
        }

        // 取指
        let instr_word = mem.load32(self.pc);
        // 使用配置的解码器解码
        let decoded = self.decoder.decode(instr_word);

        // 保存当前 PC（用于计算返回地址等）
        let current_pc = self.pc;

        // 默认顺序执行
        self.pc = self.pc.wrapping_add(4);

        // 执行指令
        self.execute(mem, decoded, current_pc);

        self.state
    }

    /// 运行多条指令
    ///
    /// # 参数
    ///
    /// * `mem` - 内存接口
    /// * `max_instructions` - 最大执行指令数
    ///
    /// # 返回
    ///
    /// 执行的指令数量和最终 CPU 状态
    ///
    /// # 停止条件
    ///
    /// - 达到最大指令数
    /// - 遇到 ECALL/EBREAK
    /// - 遇到非法指令
    pub fn run(&mut self, mem: &mut dyn Memory, max_instructions: u64) -> (u64, CpuState) {
        let mut executed = 0;
        for _ in 0..max_instructions {
            let state = self.step(mem);
            executed += 1;
            if state != CpuState::Running {
                return (executed, state);
            }
        }
        (executed, self.state)
    }

    /// 执行已解码的指令，委托到分 ISA 的执行单元
    fn execute(&mut self, mem: &mut dyn Memory, decoded: DecodedInstr, current_pc: u32) {
        let instr = decoded.instr;

        if exu::rv32i::execute(self, mem, instr, current_pc) {
            return;
        }

        if exu::rv32m::execute(self, instr) {
            return;
        }

        if exu::rv32f::execute(self, mem, instr) {
            return;
        }

        if exu::zicsr::execute(self, instr) {
            return;
        }

        if exu::priv_instr::execute(self, instr) {
            return;
        }

        match instr {
            RvInstr::Illegal { raw } => {
                self.state = CpuState::IllegalInstruction(raw);
            }
            RvInstr::Custom { extension, opcode, raw, fields } => {
                let _ = (extension, opcode, fields);
                self.state = CpuState::IllegalInstruction(raw);
            }
            _ => {
                self.state = CpuState::IllegalInstruction(decoded.raw);
            }
        }
    }

    /// 打印所有存在的状态（用于调试）
    ///
    /// 输出内容包括：
    /// - PC 和 CPU 状态
    /// - 特权级
    /// - 整数寄存器 x0-x31
    /// - 浮点寄存器 f0-f31（如果启用 F 扩展）
    /// - 向量寄存器 v0-v31（如果启用 V 扩展）
    /// - 所有已注册的 CSR
    pub fn dump_regs(&self) {
        println!("═══════════════════════════════════════════════════════════════════");
        println!("CPU Status Dump");
        println!("═══════════════════════════════════════════════════════════════════");
        
        // PC 和状态
        println!("PC: 0x{:08x}  State: {:?}  Privilege: {:?}", 
                 self.pc, self.state, self.status.privilege);
        println!();
        
        // 整数寄存器
        println!("─── Integer Registers (x0-x31) ───────────────────────────────────");
        for i in 0..32 {
            if i % 4 == 0 {
                print!("  ");
            }
            print!("x{:02}: 0x{:08x}  ", i, self.read_reg(i as u8));
            if i % 4 == 3 {
                println!();
            }
        }
        
        // 浮点寄存器（如果存在）
        if let Some(fp) = &self.status.fp {
            println!();
            println!("─── Floating-Point Registers (f0-f31) ────────────────────────────");
            for i in 0..32 {
                if i % 4 == 0 {
                    print!("  ");
                }
                let bits = fp.read(i as u8);
                let f = f32::from_bits(bits);
                print!("f{:02}: {:12.6} ", i, f);
                if i % 4 == 3 {
                    println!();
                }
            }
            // 再打印一遍十六进制形式
            println!("  (hex):");
            for i in 0..32 {
                if i % 4 == 0 {
                    print!("  ");
                }
                print!("f{:02}: 0x{:08x}  ", i, fp.read(i as u8));
                if i % 4 == 3 {
                    println!();
                }
            }
        }
        
        // 向量寄存器（如果存在）
        if let Some(vec) = &self.status.vec {
            println!();
            println!("─── Vector Registers (v0-v31, VLEN=128) ──────────────────────────");
            for i in 0..32 {
                let v = vec.read(i as u8);
                print!("  v{:02}: ", i);
                for b in v.iter().rev() {
                    print!("{:02x}", b);
                }
                println!();
            }
        }
        
        // CSR 寄存器（按地址排序）
        let csr_snapshot = self.status.csr.snapshot();
        if !csr_snapshot.is_empty() {
            println!();
            println!("─── Control and Status Registers (CSR) ───────────────────────────");
            let mut csr_list: Vec<_> = csr_snapshot.iter().collect();
            csr_list.sort_by_key(|(addr, _)| *addr);
            
            for (i, (addr, value)) in csr_list.iter().enumerate() {
                print!("  0x{:03x}: 0x{:08x}", addr, value);
                if i % 3 == 2 {
                    println!();
                } else {
                    print!("  ");
                }
            }
            // 如果最后一行没有换行，补上
            if csr_list.len() % 3 != 0 {
                println!();
            }
        }
        
        println!("═══════════════════════════════════════════════════════════════════");
    }
}

impl Default for CpuCore {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::FlatMemory;

    /// 将指令写入内存
    fn write_instr(mem: &mut FlatMemory, addr: u32, instr: u32) {
        mem.store32(addr, instr);
    }

    #[test]
    fn test_addi() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, 42
        write_instr(&mut mem, 0, 0x02A00093);
        cpu.step(&mut mem);

        assert_eq!(cpu.read_reg(1), 42);
        assert_eq!(cpu.pc(), 4);
    }

    #[test]
    fn test_add() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, 10
        write_instr(&mut mem, 0, 0x00A00093);
        // addi x2, x0, 20
        write_instr(&mut mem, 4, 0x01400113);
        // add x3, x1, x2
        write_instr(&mut mem, 8, 0x002081B3);

        cpu.step(&mut mem);
        cpu.step(&mut mem);
        cpu.step(&mut mem);

        assert_eq!(cpu.read_reg(1), 10);
        assert_eq!(cpu.read_reg(2), 20);
        assert_eq!(cpu.read_reg(3), 30);
    }

    #[test]
    fn test_sub() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, 100
        write_instr(&mut mem, 0, 0x06400093);
        // addi x2, x0, 30
        write_instr(&mut mem, 4, 0x01E00113);
        // sub x3, x1, x2
        write_instr(&mut mem, 8, 0x402081B3);

        cpu.run(&mut mem, 3);

        assert_eq!(cpu.read_reg(3), 70);
    }

    #[test]
    fn test_lw_sw() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, 0x42 (将 0x42 存入 x1)
        write_instr(&mut mem, 0, 0x04200093);
        // addi x2, x0, 100 (x2 = 100, 作为基地址)
        write_instr(&mut mem, 4, 0x06400113);
        // sw x1, 0(x2) (将 x1 存入 mem[100])
        write_instr(&mut mem, 8, 0x00112023);
        // lw x3, 0(x2) (从 mem[100] 加载到 x3)
        write_instr(&mut mem, 12, 0x00012183);

        cpu.run(&mut mem, 4);

        assert_eq!(cpu.read_reg(3), 0x42);
        assert_eq!(mem.load32(100), 0x42);
    }

    #[test]
    fn test_beq_taken() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, 5
        write_instr(&mut mem, 0, 0x00500093);
        // addi x2, x0, 5
        write_instr(&mut mem, 4, 0x00500113);
        // beq x1, x2, 8 (跳转到 PC+8=12)
        write_instr(&mut mem, 8, 0x00208463);
        // addi x3, x0, 1 (如果不跳转则执行)
        write_instr(&mut mem, 12, 0x00100193);

        cpu.run(&mut mem, 3);

        // beq 应该跳转到地址 16 (8 + 8)
        assert_eq!(cpu.pc(), 16);
        // x3 不应该被修改（因为跳过了地址 12 的指令）
        assert_eq!(cpu.read_reg(3), 0);
    }

    #[test]
    fn test_beq_not_taken() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, 5
        write_instr(&mut mem, 0, 0x00500093);
        // addi x2, x0, 10
        write_instr(&mut mem, 4, 0x00A00113);
        // beq x1, x2, 8 (不跳转，因为 x1 != x2)
        write_instr(&mut mem, 8, 0x00208463);
        // addi x3, x0, 1 (应该执行)
        write_instr(&mut mem, 12, 0x00100193);

        cpu.run(&mut mem, 4);

        assert_eq!(cpu.read_reg(3), 1);
    }

    #[test]
    fn test_jal() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // jal x1, 8 (跳转到 PC+8=8，x1 = PC+4=4)
        write_instr(&mut mem, 0, 0x008000EF);

        cpu.step(&mut mem);

        assert_eq!(cpu.read_reg(1), 4); // 返回地址
        assert_eq!(cpu.pc(), 8); // 跳转目标
    }

    #[test]
    fn test_lui() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // lui x1, 0x12345
        write_instr(&mut mem, 0, 0x123450B7);

        cpu.step(&mut mem);

        assert_eq!(cpu.read_reg(1), 0x12345000);
    }

    #[test]
    fn test_auipc() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0x100);

        // auipc x1, 0x12345 (x1 = PC + (0x12345 << 12))
        write_instr(&mut mem, 0x100, 0x12345097);

        cpu.step(&mut mem);

        assert_eq!(cpu.read_reg(1), 0x100 + 0x12345000);
    }

    #[test]
    fn test_x0_always_zero() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x0, x0, 42 (尝试写入 x0)
        write_instr(&mut mem, 0, 0x02A00013);

        cpu.step(&mut mem);

        // x0 应该仍然是 0
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_ecall() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // 设置 trap handler 地址
        cpu.csr_write(0x305, 0x100); // mtvec = 0x100

        // ecall at PC=0
        write_instr(&mut mem, 0, 0x00000073);

        let state = cpu.step(&mut mem);

        assert_eq!(state, CpuState::Running); // trap 后继续运行
        assert_eq!(cpu.pc(), 0x100); // 跳转到 mtvec
        assert_eq!(cpu.csr_read(0x341), 0); // mepc = 原 PC
        assert_eq!(cpu.csr_read(0x342), 11); // mcause = 11 (ecall from M-mode)
    }

    #[test]
    fn test_ebreak() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // 设置 trap handler 地址
        cpu.csr_write(0x305, 0x200); // mtvec = 0x200

        // ebreak at PC=0
        write_instr(&mut mem, 0, 0x00100073);

        let state = cpu.step(&mut mem);

        assert_eq!(state, CpuState::Running); // trap 后继续运行
        assert_eq!(cpu.pc(), 0x200); // 跳转到 mtvec
        assert_eq!(cpu.csr_read(0x341), 0); // mepc = 原 PC
        assert_eq!(cpu.csr_read(0x342), 3); // mcause = 3 (breakpoint)
    }

    #[test]
    fn test_shift_instructions() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, 0x10 (x1 = 16)
        write_instr(&mut mem, 0, 0x01000093);
        // slli x2, x1, 2 (x2 = 16 << 2 = 64)
        write_instr(&mut mem, 4, 0x00209113);
        // srli x3, x2, 1 (x3 = 64 >> 1 = 32)
        write_instr(&mut mem, 8, 0x00115193);

        cpu.run(&mut mem, 3);

        assert_eq!(cpu.read_reg(1), 16);
        assert_eq!(cpu.read_reg(2), 64);
        assert_eq!(cpu.read_reg(3), 32);
    }

    #[test]
    fn test_slt() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // addi x1, x0, -5 (x1 = -5)
        write_instr(&mut mem, 0, 0xFFB00093);
        // addi x2, x0, 10 (x2 = 10)
        write_instr(&mut mem, 4, 0x00A00113);
        // slt x3, x1, x2 (x3 = 1, 因为 -5 < 10)
        write_instr(&mut mem, 8, 0x0020A1B3);

        cpu.run(&mut mem, 3);

        assert_eq!(cpu.read_reg(3), 1);
    }

    #[test]
    fn test_simple_loop() {
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuCore::new(0);

        // 设置 trap handler 地址
        cpu.csr_write(0x305, 0x100); // mtvec = 0x100

        // 简单循环：计算 1+2+3 = 6
        // addi x1, x0, 0     # x1 = sum = 0
        write_instr(&mut mem, 0, 0x00000093);
        // addi x2, x0, 1     # x2 = i = 1
        write_instr(&mut mem, 4, 0x00100113);
        // addi x3, x0, 4     # x3 = limit = 4
        write_instr(&mut mem, 8, 0x00400193);
        // loop:
        // add x1, x1, x2     # sum += i
        write_instr(&mut mem, 12, 0x002080B3);
        // addi x2, x2, 1     # i++
        write_instr(&mut mem, 16, 0x00110113);
        // blt x2, x3, -8     # if i < limit goto loop (PC-8)
        write_instr(&mut mem, 20, 0xFE314CE3);
        // ecall              # 结束
        write_instr(&mut mem, 24, 0x00000073);

        // 运行直到 PC 跳转到 trap handler
        let mut executed = 0;
        while cpu.pc() != 0x100 && executed < 100 {
            cpu.step(&mut mem);
            executed += 1;
        }

        assert_eq!(cpu.read_reg(1), 6); // 1 + 2 + 3 = 6
        assert_eq!(cpu.pc(), 0x100); // 跳转到 trap handler
        assert_eq!(cpu.csr_read(0x341), 24); // mepc = ecall 指令地址
        assert_eq!(cpu.csr_read(0x342), 11); // mcause = ecall from M-mode
        assert!(executed < 100);
    }

    #[test]
    fn test_cpu_builder_basic() {
        // 使用 CpuBuilder 创建带 M 扩展的 CPU
        let cpu = CpuBuilder::new(0x1000)
            .with_m_extension()
            .build()
            .expect("配置无冲突");
        
        assert_eq!(cpu.pc(), 0x1000);
        assert_eq!(cpu.state(), CpuState::Running);
    }

    #[test]
    fn test_cpu_builder_with_f_extension() {
        // 使用 CpuBuilder 创建带 F 扩展的 CPU
        let cpu = CpuBuilder::new(0)
            .with_f_extension()
            .build()
            .expect("配置无冲突");
        
        // 验证 F 扩展 CSR 已注册 (fflags=0x001, frm=0x002, fcsr=0x003)
        // 通过 status snapshot 检查
        let snapshot = cpu.snapshot();
        // CSR 应该包含 F 扩展的 CSR
        assert!(snapshot.csr.contains_key(&0x001), "fflags 应已注册");
        assert!(snapshot.csr.contains_key(&0x002), "frm 应已注册");
        assert!(snapshot.csr.contains_key(&0x003), "fcsr 应已注册");
    }

    #[test]
    fn test_cpu_builder_with_v_extension() {
        // 使用 CpuBuilder 创建带 V 扩展的 CPU
        let cpu = CpuBuilder::new(0)
            .with_v_extension()
            .build()
            .expect("配置无冲突");
        
        let snapshot = cpu.snapshot();
        // V 扩展 CSR: vstart=0x008, vl=0xC20, vtype=0xC21
        assert!(snapshot.csr.contains_key(&0x008), "vstart 应已注册");
        assert!(snapshot.csr.contains_key(&0xC20), "vl 应已注册");
        assert!(snapshot.csr.contains_key(&0xC21), "vtype 应已注册");
        // vlenb 应该有默认值 16 (VLEN=128, vlenb=VLEN/8=16)
        assert_eq!(snapshot.csr.get(&0xC22), Some(&16), "vlenb 应为 16");
    }

    #[test]
    fn test_cpu_builder_m_mode_csrs() {
        // 默认启用 M-mode
        let cpu = CpuBuilder::new(0)
            .build()
            .expect("配置无冲突");
        
        let snapshot = cpu.snapshot();
        // M-mode CSR: mstatus=0x300, mepc=0x341
        assert!(snapshot.csr.contains_key(&0x300), "mstatus 应已注册");
        assert!(snapshot.csr.contains_key(&0x341), "mepc 应已注册");
    }

    #[test]
    fn test_cpu_builder_run_program() {
        // 使用 CpuBuilder 创建 CPU 并运行简单程序
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuBuilder::new(0)
            .with_m_extension()
            .build()
            .expect("配置无冲突");
        
        // 设置 trap handler
        cpu.csr_write(0x305, 0x100); // mtvec = 0x100
        
        // addi x1, x0, 42
        write_instr(&mut mem, 0, 0x02A00093);
        // addi x2, x1, 8
        write_instr(&mut mem, 4, 0x00808113);
        // ecall
        write_instr(&mut mem, 8, 0x00000073);
        
        // 运行直到 PC 跳转到 trap handler
        let mut executed = 0;
        while cpu.pc() != 0x100 && executed < 10 {
            cpu.step(&mut mem);
            executed += 1;
        }
        
        assert_eq!(cpu.read_reg(1), 42);
        assert_eq!(cpu.read_reg(2), 50);
        assert_eq!(cpu.pc(), 0x100); // 跳转到 trap handler
        assert_eq!(cpu.csr_read(0x342), 11); // mcause = ecall from M-mode
        assert_eq!(executed, 3);
    }

    #[test]
    fn test_rv32im_full_program() {
        // RV32IM 完整测试：计算阶乘 5! = 120
        // 使用乘法指令 mul
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuBuilder::new(0)
            .with_m_extension()
            .build()
            .expect("配置无冲突");
        
        // 设置 trap handler
        cpu.csr_write(0x305, 0x100); // mtvec = 0x100
        
        // 程序：计算 5! = 1*2*3*4*5 = 120
        // x1 = result = 1
        // x2 = i = 1
        // x3 = limit = 6
        // loop: result = result * i; i++; if i < limit goto loop
        
        // addi x1, x0, 1      # x1 = result = 1
        write_instr(&mut mem, 0, 0x00100093);
        // addi x2, x0, 1      # x2 = i = 1  
        write_instr(&mut mem, 4, 0x00100113);
        // addi x3, x0, 6      # x3 = limit = 6
        write_instr(&mut mem, 8, 0x00600193);
        
        // loop (addr 12):
        // mul x1, x1, x2      # result *= i (RV32M)
        // opcode=0x33, funct3=0, funct7=0x01, rd=1, rs1=1, rs2=2
        // 0000001 00010 00001 000 00001 0110011 = 0x022080B3
        write_instr(&mut mem, 12, 0x022080B3);
        
        // addi x2, x2, 1      # i++
        write_instr(&mut mem, 16, 0x00110113);
        
        // blt x2, x3, -8      # if i < limit goto loop (PC = 12)
        // imm[12|10:5]=1111111, rs2=3, rs1=2, funct3=100, imm[4:1|11]=1100, opcode=1100011
        // 要跳回 addr 12，当前 addr 20，offset = 12 - 20 = -8
        write_instr(&mut mem, 20, 0xFE314CE3);
        
        // ecall               # 结束
        write_instr(&mut mem, 24, 0x00000073);
        
        // 运行直到 PC 跳转到 trap handler
        let mut executed = 0;
        while cpu.pc() != 0x100 && executed < 50 {
            cpu.step(&mut mem);
            executed += 1;
        }
        
        // 验证结果
        assert_eq!(cpu.read_reg(1), 120, "5! = 120");
        assert_eq!(cpu.read_reg(2), 6, "i 最终等于 6");
        assert_eq!(cpu.pc(), 0x100); // 跳转到 trap handler
        assert_eq!(cpu.csr_read(0x342), 11); // mcause = ecall from M-mode
        println!("RV32IM 阶乘测试: 5! = {}, 执行了 {} 条指令", cpu.read_reg(1), executed);
    }

    #[test]
    fn test_rv32im_div_rem() {
        // RV32M 除法和取余测试
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuBuilder::new(0)
            .with_m_extension()
            .build()
            .expect("配置无冲突");
        
        // 设置 trap handler
        cpu.csr_write(0x305, 0x100); // mtvec = 0x100
        
        // 计算 17 / 5 = 3 余 2
        
        // addi x1, x0, 17     # x1 = 17
        write_instr(&mut mem, 0, 0x01100093);
        // addi x2, x0, 5      # x2 = 5
        write_instr(&mut mem, 4, 0x00500113);
        
        // div x3, x1, x2      # x3 = 17 / 5 = 3
        // funct7=0x01, rs2=2, rs1=1, funct3=100, rd=3, opcode=0x33
        // 0000001 00010 00001 100 00011 0110011 = 0x0220C1B3
        write_instr(&mut mem, 8, 0x0220C1B3);
        
        // rem x4, x1, x2      # x4 = 17 % 5 = 2
        // funct7=0x01, rs2=2, rs1=1, funct3=110, rd=4, opcode=0x33
        // 0000001 00010 00001 110 00100 0110011 = 0x0220E233
        write_instr(&mut mem, 12, 0x0220E233);
        
        // 验证: x3 * x2 + x4 == x1 (商*除数+余数=被除数)
        // mul x5, x3, x2      # x5 = 3 * 5 = 15
        write_instr(&mut mem, 16, 0x022181B3); // 这里 rd=3 改为 rd=5
        // 更正: mul x5, x3, x2
        // 0000001 00010 00011 000 00101 0110011 = 0x022182B3
        write_instr(&mut mem, 16, 0x022182B3);
        
        // add x6, x5, x4      # x6 = 15 + 2 = 17
        write_instr(&mut mem, 20, 0x00428333);
        
        // ecall
        write_instr(&mut mem, 24, 0x00000073);
        
        // 运行直到 PC 跳转到 trap handler
        let mut executed = 0;
        while cpu.pc() != 0x100 && executed < 10 {
            cpu.step(&mut mem);
            executed += 1;
        }
        
        assert_eq!(cpu.read_reg(1), 17, "被除数");
        assert_eq!(cpu.read_reg(2), 5, "除数");
        assert_eq!(cpu.read_reg(3), 3, "商 = 17 / 5 = 3");
        assert_eq!(cpu.read_reg(4), 2, "余数 = 17 % 5 = 2");
        assert_eq!(cpu.read_reg(5), 15, "商 * 除数 = 15");
        assert_eq!(cpu.read_reg(6), 17, "商*除数+余数 = 被除数");
        assert_eq!(cpu.pc(), 0x100); // 跳转到 trap handler
        println!("RV32IM 除法测试: 17 / 5 = {} 余 {}", cpu.read_reg(3), cpu.read_reg(4));
    }

    #[test]
    fn test_rv32im_fibonacci() {
        // 计算斐波那契数列 F(10) = 55
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuBuilder::new(0)
            .with_m_extension()
            .build()
            .expect("配置无冲突");
        
        // 设置 trap handler
        cpu.csr_write(0x305, 0x100); // mtvec = 0x100
        
        // F(1)=1, F(2)=1, F(3)=2, F(4)=3, F(5)=5, F(6)=8, F(7)=13, F(8)=21, F(9)=34, F(10)=55
        // x1 = a = 1 (F1), x2 = b = 1 (F2), 循环8次得到 F(10)
        // x3 = counter, x4 = limit, x5 = temp
        
        // addi x1, x0, 1      # a = 1 (F1)
        write_instr(&mut mem, 0, 0x00100093);
        // addi x2, x0, 1      # b = 1 (F2) 
        write_instr(&mut mem, 4, 0x00100113);
        // addi x3, x0, 2      # counter = 2 (已经计算到 F2)
        write_instr(&mut mem, 8, 0x00200193);
        // addi x4, x0, 10     # limit = 10 (计算到 F10 时停止)
        write_instr(&mut mem, 12, 0x00A00213);
        
        // loop@16: (当 counter < limit 时执行)
        // add x5, x1, x2      # temp = a + b
        write_instr(&mut mem, 16, 0x002082B3);
        // addi x1, x2, 0      # a = b
        write_instr(&mut mem, 20, 0x00010093);
        // addi x2, x5, 0      # b = temp (现在 b = F(counter+1))
        write_instr(&mut mem, 24, 0x00028113);
        // addi x3, x3, 1      # counter++
        write_instr(&mut mem, 28, 0x00118193);
        // blt x3, x4, -16     # if counter < limit goto loop
        // 当前 PC=32, 目标 PC=16, offset = -16
        write_instr(&mut mem, 32, 0xFE41C8E3);
        
        // ecall
        write_instr(&mut mem, 36, 0x00000073);
        
        // 运行直到 PC 跳转到 trap handler
        let mut executed = 0;
        while cpu.pc() != 0x100 && executed < 100 {
            cpu.step(&mut mem);
            executed += 1;
        }
        
        // 当循环结束时 counter=10, b = F(10) = 55
        println!("斐波那契测试: F(10) = {}, 执行了 {} 条指令", cpu.read_reg(2), executed);
        println!("x1={}, x2={}, x3={}, x4={}", cpu.read_reg(1), cpu.read_reg(2), cpu.read_reg(3), cpu.read_reg(4));
        
        assert_eq!(cpu.read_reg(2), 55, "F(10) = 55");
        assert_eq!(cpu.pc(), 0x100); // 跳转到 trap handler
    }

    #[test]
    fn test_zicsr_basic() {
        // 测试 CSR 指令的基本功能
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuBuilder::new(0)
            .with_zicsr_extension()
            .build()
            .expect("配置无冲突");
        
        // 测试程序：
        // 1. 读写 mscratch CSR
        // 2. 使用置位/清位操作
        
        // addi x1, x0, 0x55   # x1 = 0x55
        write_instr(&mut mem, 0, 0x05500093);
        
        // csrrw x2, mscratch, x1  # x2 = old mscratch (0), mscratch = 0x55
        // CSR mscratch = 0x340
        // csrrw: csr=0x340, rs1=x1, rd=x2, funct3=001, opcode=0x73
        // = 0x34009173
        write_instr(&mut mem, 4, 0x34009173);
        
        // csrrs x3, mscratch, x0  # x3 = mscratch (0x55), 不修改 (rs1=x0)
        // = 0x34002173 (rd=x3, rs1=x0, funct3=010)
        write_instr(&mut mem, 8, 0x340021F3);
        
        // addi x4, x0, 0x0F   # x4 = 0x0F (用于置位)
        write_instr(&mut mem, 12, 0x00F00213);
        
        // csrrs x5, mscratch, x4  # x5 = 0x55, mscratch = 0x55 | 0x0F = 0x5F
        // = 0x34022173 (rd=x5, rs1=x4, funct3=010)
        write_instr(&mut mem, 16, 0x340222F3);
        
        // csrrs x6, mscratch, x0  # x6 = mscratch (0x5F)
        write_instr(&mut mem, 20, 0x34002373);
        
        // addi x7, x0, 0x50   # x7 = 0x50 (用于清位)
        write_instr(&mut mem, 24, 0x05000393);
        
        // csrrc x8, mscratch, x7  # x8 = 0x5F, mscratch = 0x5F & ~0x50 = 0x0F
        // csrrc: funct3=011
        write_instr(&mut mem, 28, 0x3403B473);
        
        // csrrs x9, mscratch, x0  # x9 = mscratch (0x0F)
        write_instr(&mut mem, 32, 0x340024F3);
        
        // csrrwi x10, mscratch, 0x1F  # x10 = 0x0F, mscratch = 0x1F
        // csrrwi: zimm=0x1F, funct3=101
        write_instr(&mut mem, 36, 0x340FD573);
        
        // csrrs x11, mscratch, x0  # x11 = mscratch (0x1F)
        write_instr(&mut mem, 40, 0x340025F3);
        
        // ecall
        write_instr(&mut mem, 44, 0x00000073);
        
        // 设置 trap handler
        cpu.csr_write(0x305, 0x200); // mtvec = 0x200
        
        // 运行直到 PC 跳转到 trap handler
        let mut executed = 0;
        while cpu.pc() != 0x200 && executed < 20 {
            cpu.step(&mut mem);
            executed += 1;
        }
        
        println!("Zicsr 测试: 执行了 {} 条指令", executed);
        println!("x1={:#x}, x2={:#x}, x3={:#x}", cpu.read_reg(1), cpu.read_reg(2), cpu.read_reg(3));
        println!("x4={:#x}, x5={:#x}, x6={:#x}", cpu.read_reg(4), cpu.read_reg(5), cpu.read_reg(6));
        println!("x7={:#x}, x8={:#x}, x9={:#x}", cpu.read_reg(7), cpu.read_reg(8), cpu.read_reg(9));
        println!("x10={:#x}, x11={:#x}", cpu.read_reg(10), cpu.read_reg(11));
        
        // 验证结果
        assert_eq!(cpu.read_reg(1), 0x55, "x1 = 0x55");
        assert_eq!(cpu.read_reg(2), 0, "x2 = old mscratch = 0");
        assert_eq!(cpu.read_reg(3), 0x55, "x3 = mscratch after write = 0x55");
        assert_eq!(cpu.read_reg(5), 0x55, "x5 = mscratch before set = 0x55");
        assert_eq!(cpu.read_reg(6), 0x5F, "x6 = mscratch after set = 0x5F");
        assert_eq!(cpu.read_reg(8), 0x5F, "x8 = mscratch before clear = 0x5F");
        assert_eq!(cpu.read_reg(9), 0x0F, "x9 = mscratch after clear = 0x0F");
        assert_eq!(cpu.read_reg(10), 0x0F, "x10 = mscratch before csrrwi = 0x0F");
        assert_eq!(cpu.read_reg(11), 0x1F, "x11 = mscratch after csrrwi = 0x1F");
        assert_eq!(cpu.pc(), 0x200); // 跳转到 trap handler
    }

    #[test]
    fn test_take_trap_basic() {
        // 测试 take_trap 方法的基本功能
        use crate::cpu::csr_def::*;
        
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuBuilder::new(0x1000)
            .with_zicsr_extension()
            .build()
            .expect("配置无冲突");
        
        // 设置 mtvec = 0x8000_0000 (direct mode)
        cpu.status.csr_write(CSR_MTVEC, 0x8000_0000);
        
        // 设置初始 mstatus 启用中断 (MIE=1)
        cpu.status.csr_write(CSR_MSTATUS, 0x8); // MIE bit
        
        // 模拟 ecall 触发 trap
        let cause = TrapCause::EcallFromM;
        let tval = 0; // ecall 没有 tval
        
        let pc_before = cpu.pc;
        cpu.take_trap(cause, tval);
        
        // 验证 mepc = PC before trap
        assert_eq!(cpu.status.csr_read(CSR_MEPC), pc_before, "mepc should be saved PC");
        
        // 验证 mcause = 11 (ecall from M-mode)
        assert_eq!(cpu.status.csr_read(CSR_MCAUSE), 11, "mcause should be 11");
        
        // 验证 mtval = 0
        assert_eq!(cpu.status.csr_read(CSR_MTVAL), 0, "mtval should be 0");
        
        // 验证 mstatus: MPIE=old MIE (1), MIE=0, MPP=3 (Machine)
        let mstatus = cpu.status.csr_read(CSR_MSTATUS);
        assert_eq!((mstatus >> 7) & 1, 1, "MPIE should be 1 (old MIE)");
        assert_eq!((mstatus >> 3) & 1, 0, "MIE should be 0 (disabled)");
        assert_eq!((mstatus >> 11) & 3, 3, "MPP should be 3 (Machine)");
        
        // 验证 PC 跳转到 mtvec
        assert_eq!(cpu.pc, 0x8000_0000, "PC should jump to mtvec");
        
        // 验证特权级仍然是 Machine
        assert_eq!(cpu.status.privilege, PrivilegeMode::Machine);
        
        println!("take_trap 基本测试通过!");
    }

    #[test]
    fn test_take_trap_vectored() {
        // 测试向量模式中断
        use crate::cpu::csr_def::*;
        
        let mut cpu = CpuBuilder::new(0x1000)
            .with_zicsr_extension()
            .build()
            .expect("配置无冲突");
        
        // 设置 mtvec = 0x8000_0001 (vectored mode, base=0x8000_0000)
        cpu.status.csr_write(CSR_MTVEC, 0x8000_0001);
        
        // 测试异常 (IllegalInstruction, code=2) - 应该跳转到 base
        cpu.pc = 0x1000;
        cpu.take_trap(TrapCause::IllegalInstruction, 0xDEADBEEF);
        assert_eq!(cpu.pc, 0x8000_0000, "Exception should go to base");
        assert_eq!(cpu.status.csr_read(CSR_MTVAL), 0xDEADBEEF, "mtval should contain bad instruction");
        
        // 测试中断 (MachineTimerInterrupt, code=7) - 应该跳转到 base + 4*7
        cpu.pc = 0x2000;
        cpu.take_trap(TrapCause::MachineTimerInterrupt, 0);
        assert_eq!(cpu.pc, 0x8000_0000 + 4 * 7, "Interrupt should go to base + 4*cause");
        
        println!("take_trap 向量模式测试通过!");
    }

    #[test]
    fn test_mret_basic() {
        // 测试 MRET 指令的基本功能
        use crate::cpu::csr_def::*;
        use crate::isa::MRET_ENCODING;
        
        let mut mem = FlatMemory::new(4096, 0);
        let mut cpu = CpuBuilder::new(0)
            .with_zicsr_extension()
            .with_priv_extension()
            .build()
            .expect("配置无冲突");
        
        // 模拟 trap handler 准备返回的状态
        // mepc = 0x1000 (返回地址)
        cpu.status.csr_write(CSR_MEPC, 0x1000);
        // mstatus: MPIE=1, MIE=0, MPP=0 (User mode)
        // MPIE at bit 7, MPP at bits 11-12
        let mstatus = 1 << 7; // MPIE=1, MIE=0, MPP=0
        cpu.status.csr_write(CSR_MSTATUS, mstatus);
        
        // 放置 MRET 指令
        write_instr(&mut mem, 0, MRET_ENCODING);
        
        // 执行 MRET
        cpu.step(&mut mem);
        
        // 验证结果
        // PC 应该跳转到 mepc (0x1000)
        assert_eq!(cpu.pc(), 0x1000, "PC should be mepc");
        
        // 特权级应该变为 MPP 值 (User mode)
        assert_eq!(cpu.privilege(), PrivilegeMode::User, "Should return to User mode");
        
        // mstatus: MIE 应该恢复为 MPIE (1), MPIE 应该为 1, MPP 应该为 0
        let new_mstatus = cpu.status.csr_read(CSR_MSTATUS);
        assert_eq!((new_mstatus >> 3) & 1, 1, "MIE should be restored to MPIE (1)");
        assert_eq!((new_mstatus >> 7) & 1, 1, "MPIE should be 1");
        assert_eq!((new_mstatus >> 11) & 3, 0, "MPP should be 0 (User)");
        
        println!("MRET 基本测试通过!");
    }

    #[test]
    fn test_trap_and_return_cycle() {
        // 测试完整的 trap -> handler -> mret 周期
        use crate::cpu::csr_def::*;
        use crate::isa::MRET_ENCODING;
        
        let mut mem = FlatMemory::new(0x10000, 0);
        let mut cpu = CpuBuilder::new(0x1000)
            .with_zicsr_extension()
            .with_priv_extension()
            .build()
            .expect("配置无冲突");
        
        // 设置 mtvec 指向 trap handler
        let handler_addr = 0x8000u32;
        cpu.status.csr_write(CSR_MTVEC, handler_addr);
        
        // 设置初始 mstatus: MIE=1 (中断使能)
        cpu.status.csr_write(CSR_MSTATUS, 1 << 3);
        
        // 在 handler 地址放置 MRET
        write_instr(&mut mem, handler_addr, MRET_ENCODING);
        
        // 保存原始 PC
        let original_pc = cpu.pc();
        
        // 触发 trap (模拟 ecall from M-mode)
        cpu.take_trap(TrapCause::EcallFromM, 0);
        
        // 验证 trap 后状态
        assert_eq!(cpu.pc(), handler_addr, "Should jump to handler");
        assert_eq!(cpu.status.csr_read(CSR_MEPC), original_pc, "mepc should be saved PC");
        assert_eq!(cpu.status.csr_read(CSR_MCAUSE), 11, "mcause should be 11 (EcallFromM)");
        
        // 执行 handler 中的 MRET
        cpu.step(&mut mem);
        
        // 验证返回后状态
        assert_eq!(cpu.pc(), original_pc, "Should return to original PC");
        
        // MIE 应该恢复 (因为 MPIE 是 1)
        let mstatus = cpu.status.csr_read(CSR_MSTATUS);
        assert_eq!((mstatus >> 3) & 1, 1, "MIE should be restored");
        
        println!("Trap/Return 周期测试通过!");
    }

    #[test]
    fn test_wfi() {
        // 测试 WFI 指令
        use crate::isa::WFI_ENCODING;
        
        let mut mem = FlatMemory::new(1024, 0);
        let mut cpu = CpuBuilder::new(0)
            .with_priv_extension()
            .build()
            .expect("配置无冲突");
        
        // 放置 WFI 指令
        write_instr(&mut mem, 0, WFI_ENCODING);
        
        // 执行 WFI
        let state = cpu.step(&mut mem);
        
        // 应该进入 WaitForInterrupt 状态
        assert_eq!(state, CpuState::WaitForInterrupt, "Should enter WaitForInterrupt");
        
        println!("WFI 测试通过!");
    }
}
