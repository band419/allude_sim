# allude_sim: RV32I 单线程 ISA 仿真设计

## 目标与阶段规划

### 总体目标

基于 Rust 实现一个 GPGPU ISA 仿真框架：

- **高吞吐 / 仿真速度快**：强调功能正确与仿真速度，不追求周期精确（non cycle-accurate）。
- **易于持续开发与扩展**：模块边界清晰、接口稳定，方便后续演进到多线程 / GPGPU。
- **ISA 基础为 RISC-V**：每个 GPGPU 线程基于 RISC-V ISA（当前阶段选择 RV32I）。

### 分阶段目标

1. **阶段 1：单线程 RV32I 模型** （当前阶段）
   - 定义指令、寄存器、内存等抽象。
   - 实现一个快但 non cycle-accurate 的执行引擎。
   - 通过简单示例程序验证 ISA 执行路径。
2. **阶段 2：多线程 / warp 级执行模型**
   - 将多个单线程上下文组织成 warp / wavefront。
   - 引入 SIMT 特性（执行掩码、分支栈等）与简单调度器接口。
3. **阶段 3：GPGPU 特性扩展**
   - 支持 block / grid 结构、共享内存、同步原语（barrier 等）。
   - 设计可扩展的缓存 / DRAM / 特殊存储（常量内存等）抽象。

本设计文档主要聚焦 **阶段 1：基于 RV32I 的单线程 ISA 仿真内核**。

---

## 项目结构规划

以当前 Cargo 工程为基础，对 `src/` 目录进行模块化划分：

```text
Cargo.toml
src/
  lib.rs          # 库入口：导出核心模块
  main.rs         # CLI / 示例入口
  isa.rs          # RISC-V ISA 相关定义与解码
  cpu.rs          # 单线程 CPU 核心上下文与执行引擎
  memory.rs       # 内存抽象与简单实现
```

### Cargo 配置

在 `Cargo.toml` 中引入库与可执行的区分：

```toml
[package]
name = "allude_sim"
version = "0.1.0"
edition = "2024"

[lib]
name = "allude_sim"
path = "src/lib.rs"

[[bin]]
name = "allude_sim_cli"
path = "src/main.rs"

[dependencies]
```

`src/lib.rs` 统一导出核心模块：

```rust
pub mod isa;
pub mod cpu;
pub mod memory;
```

`src/main.rs` 作为 CLI 或最小示例入口，主要负责：

- 构造 `FlatMemory` 并写入简单测试程序。
- 初始化 `CpuCore`，设置初始 PC。
- 调用 `run` 执行若干条指令。
- 打印寄存器/内存状态以验证行为。

---

## 阶段 1：单线程 RV32I 模型设计

本阶段目标：实现一个 **功能正确**、**结构清晰**、**易扩展** 的单线程 RV32I 仿真引擎，以支持后续演化到 GPGPU。

### 模块划分

1. `isa.rs`：RISC-V ISA 抽象与解码
   - `enum RvInstr`：内部表示的已解码指令。
   - `struct DecodedInstr { raw: u32, instr: RvInstr }`：包含原始编码与语义信息。
   - `fn decode(raw: u32) -> DecodedInstr`：从 32-bit 指令字解码到语义结构。

2. `cpu.rs`：CPU 核心与执行引擎
   - `struct CpuCore`：单线程执行上下文。
   - `impl CpuCore`：提供 `step` / `run` 等执行接口。

3. `memory.rs`：内存抽象层
   - `trait Memory`：对上层屏蔽具体内存实现。
   - `struct FlatMemory`：简单线性内存实现（基于 `Vec<u8>`），用于功能验证。

---

## ISA 范围与抽象

### 选择 RV32I

当前阶段采用 **RV32I** 作为基础 ISA：

- 地址空间与寄存器宽度为 32 位。
- 指令定长 32-bit（暂不支持压缩指令 C 扩展）。
- 重点实现整数算术 / 逻辑、跳转 / 分支、访存指令。

未来若需要，可以在保持接口兼容的前提下增加：

- M 扩展（乘除）、A 扩展（原子）、F/D 扩展（浮点）等。
- GPU 专用扩展指令集（纹理采样、专用 load/store 等）。

### 指令语义表示：`RvInstr`

为保证扩展性与可读性，使用语义化枚举而非简单 opcode：

```rust
pub enum RvInstr {
    Add  { rd: u8, rs1: u8, rs2: u8 },
    Addi { rd: u8, rs1: u8, imm: i32 },
    Lw   { rd: u8, rs1: u8, offset: i32 },
    Sw   { rs1: u8, rs2: u8, offset: i32 },
    Beq  { rs1: u8, rs2: u8, offset: i32 },
    Jal  { rd: u8, offset: i32 },
    // ... 其他 RV32I 指令
}
```

这种表示方式的优点：

- 解码阶段一次性做完字段提取与符号扩展；
- 执行阶段代码清晰、分支少，有利于维护和优化；
- 添加新指令只需扩展枚举与匹配逻辑，不影响整体结构。

### 指令集覆盖范围

初步实现范围：

- **算术/逻辑指令**：
  - `ADD, SUB, AND, OR, XOR, SLT, SLTU, SLL, SRL, SRA`
- **立即数指令**：
  - `ADDI, ANDI, ORI, XORI, SLTI, SLTIU, SLLI, SRLI, SRAI`
- **载入/存储指令**：
  - `LB, LH, LW, LBU, LHU` 以及 `SB, SH, SW`
- **控制流指令**：
  - `JAL, JALR`
  - `BEQ, BNE, BLT, BGE, BLTU, BGEU`
- **系统指令（简单处理）**：
  - `ECALL, EBREAK`：初期可视为“终止仿真”或产生错误信号。

在此基础上，后续可以逐步扩展到其他标准扩展。

### RV32I 规范约束与实现假设

根据 RISC-V Unprivileged 手册（Volume I），本项目在 RV32I 阶段采用以下约束与实现假设：

1. **架构状态与字节序**
   - 寄存器文件：32 个 32-bit 通用寄存器 `x0..x31`，其中 `x0` 恒为 0，所有写入 `x0` 的操作应被忽略。
   - 程序计数器：`pc` 为 32-bit 字节地址。
   - 字节序：小端序（Little-endian）。`LB/LH/LW` 等指令从内存中按小端序装载数据。

2. **指令对齐与访存对齐**
   - 指令取值：仅支持定长 32-bit 指令，要求 `pc % 4 == 0`。
   - 访存对齐：
     - `LB/LBU/SB`：字节访问无对齐要求。
     - `LH/LHU/SH`：默认要求地址按 2 字节对齐。
     - `LW/SW`：默认要求地址按 4 字节对齐。
   - 阶段 1 行为约定：
     - 如检测到未对齐的 halfword/word 访问，仿真器当前阶段可以直接报错（例如 panic）或返回显式错误，用于早期调试；
     - 未来可扩展为产生架构异常（trap）并通过 CSR/特权架构进行建模。

3. **指令编码与立即数规则**
   - 使用 RISC-V 标准的六种基本格式：R / I / S / B / U / J。
   - 解码必须按位遵循手册中的字段定义：
     - `opcode[6:0], rd[11:7], funct3[14:12], rs1[19:15], rs2[24:20], funct7[31:25]`；
   - 立即数字段拼接与符号扩展：
     - I 型：`imm[31:20]`，解码时 sign-extend 至 32-bit。
     - S 型：由 `imm[31:25]` 和 `imm[11:7]` 拼接，再 sign-extend。
     - B 型：分散在 `bit[31], bit[7], bit[30:25], bit[11:8]`，拼接后左移一位再 sign-extend。
     - U 型：高 20 位直接写入，低 12 位为 0。
     - J 型：分布在 `bit[31], bit[19:12], bit[20], bit[30:21]`，拼接后左移一位再 sign-extend。
   - 非法编码：不属于 RV32I 定义的 `opcode/funct3/funct7` 组合在解码阶段视为非法指令，当前阶段可直接报错或返回显式错误类型。

4. **算术/逻辑与移位语义**
   - 算术/逻辑类指令：
     - 所有整数运算在 32-bit 上按二进制模 $2^{32}$ 计算，不产生溢出异常；
     - `SLT/SLTI` 以有符号数比较；`SLTU/SLTIU` 以无符号数比较。
   - 移位类指令：
     - RV32I 中，移位量仅使用 `rs2[4:0]` 或 `imm[4:0]`（范围 0..31）；
     - `SRA/SRAI` 为算术右移，需要保留符号位；`SRL/SRLI` 为逻辑右移。

5. **访存指令行为**
   - 地址计算：统一使用 `addr = rs1 + offset`，其中 `offset` 为已 sign-extend 的立即数。
   - 加载指令：
     - `LB/LH/LW`：从内存取值后执行有符号扩展；
     - `LBU/LHU`：从内存取值后执行零扩展；
   - 存储指令：
     - `SB/SH/SW`：按小端序拆分对应宽度写入内存。
   - 越界访问：阶段 1 可简化为直接报错或返回错误，用于快速暴露问题；未来可细化为不同类型的异常源。

6. **控制流指令行为**
   - `JAL`：
     - 将返回地址（通常为当前指令地址 + 4）写入 `rd`；
     - `pc` 更新为 `pc + offset`（offset 为 J 型立即数左移一位后加到当前 PC）。
   - `JALR`：
     - 将返回地址写入 `rd`；
     - `pc` 更新为 `(rs1 + offset) & !1`，低 1 位强制清零（2 字节对齐）。
   - 条件分支 `BEQ/BNE/BLT/BGE/BLTU/BGEU`：
     - 根据手册约定进行有符号/无符号比较；
     - 目标地址由 B 型立即数左移一位后与当前 PC 相加得到。
   - 阶段 1 实现约定：
     - 为实现简洁，可以在 `step` 中先执行 `pc += 4`，然后在执行阶段根据分支/跳转结果重新写回 `pc`，只要整体等价即可。

7. **系统指令与异常处理策略（阶段 1）**
   - `ECALL/EBREAK`：
     - 当前阶段可简化为：设置仿真器“终止”标志或直接返回错误，由上层控制循环停止；
     - 未来可扩展为根据寄存器（如 `a7`）编码不同 syscall 或调试事件。
   - 其他异常源（非法指令、对齐错误、越界访问）：
     - 阶段 1 不实现完整的 trap/CSR 机制，而是在 CPU 执行接口中通过错误返回或 panic 显式暴露错误；
     - 后续在引入特权架构/CSR 时，可以将这些错误映射为标准异常码。

---

## CPU 核心与执行引擎设计

### `CpuCore` 寄存器与状态

单线程 CPU 核心的最小状态：

```rust
pub struct CpuCore {
    regs: [u32; 32], // x0..x31
    pc: u32,         // 程序计数器
    // 未来可扩展字段：
    // - CSR
    // - 特权级别
    // - 线程/warp 相关元数据
}
```

设计约定：

- `x0` 永远为 0，写入时丢弃，在写寄存器逻辑中统一处理。
- PC 为字节地址，初期假设所有指令 4 字节对齐（不支持 C 扩展）。
- 核心状态不依赖全局变量，方便将多个 `CpuCore` 用于多线程 / warp 结构。

### 执行接口

核心执行接口分为两级：

1. 取指 + 解码
2. 执行一条指令

示意接口：

```rust
impl CpuCore {
    pub fn step(&mut self, mem: &mut dyn Memory) {
        let instr_word = mem.load32(self.pc) as u32;
        let decoded = isa::decode(instr_word);

        // 默认顺序执行：PC += 4
        self.pc = self.pc.wrapping_add(4);

        // 执行指令，必要时可覆盖 PC（分支/跳转）
        self.execute(mem, decoded);
    }

    pub fn run(&mut self, mem: &mut dyn Memory, max_instructions: u64) {
        for _ in 0..max_instructions {
            self.step(mem);
            // TODO: 根据状态（如 ECALL/EBREAK 或自定义退出条件）中止
        }
    }
}
```

执行逻辑 `execute` 内部根据 `RvInstr` 做匹配：

- 算术/逻辑：读寄存器 → 运算 → 写回寄存器。
- 访存：根据 `rs1 + offset` 计算地址，调用 `Memory` 接口读写。
- 分支/跳转：根据条件决定是否修改 PC。
- 系统指令：更新内部状态或触发终止标志。

### 非周期精确设计（non cycle-accurate）

为了高仿真速度，本设计不追求每条指令的精确时序：

- 不建模流水线阶段（IF/ID/EX/MEM/WB）与 hazard；
- 不建模 cache miss / bank conflict 的具体时序；
- 不建模具体发射宽度与 issue queue 行为；

而是聚焦在：

- **指令语义正确性**；
- **总体执行路径尽量轻量级**（减少分支与虚函数开销）；
- **易于后续在更高层次上插入近似性能模型**（如为每条指令附加一个“估算周期数”）。

---

## 内存抽象层设计

### `Memory` trait

为方便后续接入多种内存模型（平坦 DRAM、cache 分层、共享内存等），定义统一内存接口：

```rust
pub trait Memory {
    fn load8(&self, addr: u32) -> u8;
    fn load16(&self, addr: u32) -> u16;
    fn load32(&self, addr: u32) -> u32;

    fn store8(&mut self, addr: u32, value: u8);
    fn store16(&mut self, addr: u32, value: u16);
    fn store32(&mut self, addr: u32, value: u32);
}
```

设计要点：

- 地址使用 `u32`，与 RV32I 匹配；
- 按最常见的 8/16/32-bit 访存粒度建接口；
- 不假设底层物理实现，可以是简单数组、分段、或带统计的模型。

### 简单实现：`FlatMemory`

初期可使用线性内存模型，用 `Vec<u8>` 存整个地址空间或一段工作集：

```rust
pub struct FlatMemory {
    data: Vec<u8>,
    base_addr: u32, // 可选：模拟内存映射起始地址
}
```

特点：

- 实现简单，便于先验证 ISA 逻辑；
- 后续可以替换 / 包装为更复杂的内存体系结构，而不影响 CPU 与 ISA 层代码。

---

## 向 GPGPU 版本演化的接口预留

虽然当前只实现单线程 RV32I，但需要在接口层面为 GPGPU 做预留设计：

### 1. 多线程 / warp 结构

未来可在 `CpuCore` 外侧增加：

```rust
pub struct ThreadContext {
    pub core: CpuCore,
    pub thread_id: u32,
    pub warp_id: u32,
    // 其他线程级元数据
}

pub struct Warp {
    pub threads: Vec<ThreadContext>,
    pub exec_mask: u32,
    // 分支栈、SIMT reconvergence 等
}
```

由于单个 `CpuCore` 已经过 `Memory` trait 与外部世界交互，warp 只需调度多个 `ThreadContext` 即可重用当前执行引擎。

### 2. 配置结构

为避免未来增加 GPU 相关特性时大改接口，可以定义轻量配置：

```rust
pub struct CpuConfig {
    pub enable_m_extension: bool,
    // 未来：与 GPGPU 相关的选项
    // pub has_shared_memory: bool,
    // pub vector_registers: bool,
}
```

`CpuCore` 可持有 `CpuConfig` 引用或拷贝，根据配置决定支持的指令集与行为。

### 3. 内存系统扩展

通过 `Memory` trait，可以平滑切换到：

- 带统计的 DRAM 模型（计数带宽、延迟）；
- 包含 L1/L2 cache 的多级存储；
- GPGPU 中常见的 shared memory、global memory、constant/texture memory 等抽象。

---

## 阶段 1 实现路线

阶段 1 的具体实施步骤建议如下：

1. **搭建工程骨架**
   - 修改 `Cargo.toml`，增加 `[lib]` 和 `[[bin]]` 配置。
   - 创建 `src/lib.rs` 并导出 `isa`, `cpu`, `memory` 模块。
   - 创建空的 `src/isa.rs`, `src/cpu.rs`, `src/memory.rs` 文件。

2. **实现内存抽象与简单实现**
   - 在 `memory.rs` 中定义 `Memory` trait。
   - 实现 `FlatMemory`，支持基础的 load/store 操作。

3. **实现 RV32I 解码**
   - 在 `isa.rs` 中定义 `RvInstr` 与 `DecodedInstr`。
   - 实现 `decode(raw: u32) -> DecodedInstr`，覆盖核心 RV32I 指令。

4. **实现 CPU 核心与执行引擎**
   - 在 `cpu.rs` 中定义 `CpuCore`（寄存器文件 + PC）。
   - 实现 `step`：取指 → 解码 → 默认 `pc += 4` → 执行。
   - 实现 `execute`：按 `RvInstr` 分类执行算术/访存/控制流指令。

5. **编写最小示例 / 单元测试**
   - 在 `main.rs` 中构造一个简单程序（例如：计算 `1 + 2 + 3`，结果写入某个寄存器或内存）。
   - 使用 `FlatMemory` 装载该程序，运行若干条指令，并打印寄存器状态验证正确性。
   - 后续可增加 `#[cfg(test)]` 单元测试，验证单条指令行为。

完成以上步骤后，即可得到一个结构清晰、可运行的单线程 RV32I 仿真实现，为后续多线程 / GPGPU 扩展打下基础。
