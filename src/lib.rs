//! allude_sim: RV32I 单线程 ISA 仿真库
//!
//! 本库实现了一个功能正确、结构清晰、易扩展的单线程 RV32I 仿真引擎，
//! 以支持后续演化到 GPGPU。
//!
//! # 模块结构
//!
//! - `isa`: RISC-V ISA 抽象与解码
//! - `cpu`: CPU 核心与执行引擎
//! - `memory`: 内存抽象层
//! - `sim_env`: 仿真环境（配置、ELF 加载、初始化）

pub mod cpu;
pub mod isa;
pub mod memory;
pub mod sim_env;
