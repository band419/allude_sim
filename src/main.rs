//! allude_sim CLI 示例入口
//!
//! 本文件演示如何使用 allude_sim 库进行 RV32I 仿真。

use allude_sim::cpu::{CpuCore, CpuState};
use allude_sim::memory::{FlatMemory, Memory};

fn main() {
    println!("=== allude_sim: RV32I 单线程 ISA 仿真器 ===\n");

    // 创建 64KB 内存，起始地址为 0
    let mut mem = FlatMemory::new(64 * 1024, 0);

    // 示例程序：计算 1 + 2 + 3 + ... + 10 = 55
    // 程序逻辑：
    //   x1 = sum = 0
    //   x2 = i = 1
    //   x3 = limit = 11
    // loop:
    //   sum += i
    //   i++
    //   if i < limit goto loop
    //   ecall (结束)

    let program: &[u32] = &[
        0x00000093, // addi x1, x0, 0      # x1 = sum = 0
        0x00100113, // addi x2, x0, 1      # x2 = i = 1
        0x00B00193, // addi x3, x0, 11     # x3 = limit = 11
        // loop (地址 12):
        0x002080B3, // add x1, x1, x2      # sum += i
        0x00110113, // addi x2, x2, 1      # i++
        0xFE314CE3, // blt x2, x3, -8      # if i < limit goto loop
        0x00000073, // ecall               # 结束
    ];

    // 将程序写入内存
    for (i, &instr) in program.iter().enumerate() {
        mem
            .store32((i * 4) as u32, instr)
            .expect("failed to write demo program into memory");
    }

    println!("程序已加载到内存，计算 1 + 2 + ... + 10");
    println!();

    // 初始化 CPU，PC 从 0 开始
    let mut cpu = CpuCore::new(0);

    println!("初始状态:");
    cpu.dump_regs();
    println!();

    // 运行程序
    let (executed, final_state) = cpu.run(&mut mem, 1000);

    println!("执行完毕!");
    println!("执行指令数: {}", executed);
    println!(
        "最终状态: {:?}",
        match final_state {
            CpuState::Running => "运行中",
            CpuState::IllegalInstruction(_) => "非法指令",
            CpuState::WaitForInterrupt => "等待中断 (WFI)",
            CpuState::Halted => "已停机",
        }
    );
    println!();

    println!("最终寄存器状态:");
    cpu.dump_regs();
    println!();

    // 验证结果
    let sum = cpu.read_reg(1);
    let expected = 55; // 1 + 2 + ... + 10 = 55
    println!("计算结果: x1 = {}", sum);
    println!("预期结果: {}", expected);

    if sum == expected {
        println!("✓ 验证通过!");
    } else {
        println!("✗ 验证失败!");
    }

    println!();
    println!("=== 演示 2：斐波那契数列 ===\n");
    demo_fibonacci();
}

/// 演示计算斐波那契数列
fn demo_fibonacci() {
    let mut mem = FlatMemory::new(64 * 1024, 0);

    // 计算第 10 个斐波那契数 (F(10) = 55)
    // F(0)=0, F(1)=1, F(2)=1, F(3)=2, F(4)=3, F(5)=5, F(6)=8, F(7)=13, F(8)=21, F(9)=34, F(10)=55
    //
    // 程序逻辑：
    //   x1 = a = 0      (F(0))
    //   x2 = b = 1      (F(1))
    //   x3 = i = 1      (从 1 开始计数)
    //   x4 = n = 10     (目标索引)
    // loop:
    //   if i >= n goto done   # 当 i=10 时退出，此时 x2 = F(10)
    //   x5 = a + b            # F(i+1)
    //   a = b                 # a = F(i)
    //   b = x5                # b = F(i+1)
    //   i++
    //   goto loop
    // done:
    //   ecall (x2 包含 F(n))

    let program: &[u32] = &[
        0x00000093, // 0:  addi x1, x0, 0      # a = F(0) = 0
        0x00100113, // 4:  addi x2, x0, 1      # b = F(1) = 1
        0x00100193, // 8:  addi x3, x0, 1      # i = 1
        0x00A00213, // 12: addi x4, x0, 10     # n = 10
        // loop (地址 16):
        0x0041DC63, // 16: bge x3, x4, 24      # if i >= n goto done (PC + 24 = 40)
        0x002082B3, // 20: add x5, x1, x2      # temp = a + b
        0x00010093, // 24: addi x1, x2, 0      # a = b
        0x00028113, // 28: addi x2, x5, 0      # b = temp
        0x00118193, // 32: addi x3, x3, 1      # i++
        0xFEC006E3, // 36: beq x0, x0, -20     # goto loop (PC - 20 = 16)
        // done (地址 40):
        0x00000073, // 40: ecall
    ];

    // 将程序写入内存
    for (i, &instr) in program.iter().enumerate() {
        mem
            .store32((i * 4) as u32, instr)
            .expect("failed to write fibonacci program into memory");
    }

    let mut cpu = CpuCore::new(0);

    println!("程序已加载到内存，计算 F(10) (斐波那契数列)");
    println!();

    let (executed, final_state) = cpu.run(&mut mem, 1000);

    println!("执行完毕!");
    println!("执行指令数: {}", executed);
    println!(
        "最终状态: {:?}",
        match final_state {
            CpuState::Running => "运行中",
            CpuState::IllegalInstruction(raw) => {
                println!("非法指令: 0x{:08x}", raw);
                "非法指令"
            }
            CpuState::WaitForInterrupt => "等待中断 (WFI)",
            CpuState::Halted => "已停机",
        }
    );
    println!();

    println!("最终寄存器状态:");
    cpu.dump_regs();

    // F(10) = 55 应该在 x2 (b) 中
    let result = cpu.read_reg(2);
    let expected = 55;
    println!();
    println!("计算结果: x2 = {} (F(10))", result);
    println!("预期结果: {}", expected);

    if result == expected {
        println!("✓ 验证通过!");
    } else {
        println!("✗ 验证失败!");
        // 调试信息
        println!("调试信息: a(x1)={}, b(x2)={}, i(x3)={}, n(x4)={}", 
                 cpu.read_reg(1), cpu.read_reg(2), cpu.read_reg(3), cpu.read_reg(4));
    }
}

