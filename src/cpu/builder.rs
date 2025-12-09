//! CPU 配置器
//!
//! 提供统一的接口来配置 CPU 的指令集、解码器和架构状态。
//!
//! # 示例
//!
//! ```
//! use allude_sim::cpu::{CpuBuilder, CpuCore};
//!
//! let cpu = CpuBuilder::new(0x1000)
//!     .with_m_extension()
//!     .with_f_extension()
//!     .build()
//!     .expect("配置无冲突");
//! ```

use std::sync::Arc;

use super::csr_def;
use super::status::Status;
use super::CpuCore;
use crate::isa::{IsaConfig, ConflictInfo};

/// CPU 构建器
///
/// 用于根据用户指定的指令集扩展，统一配置：
/// - 解码器 (decoder)
/// - 执行单元 (exu) — 目前由 CpuCore 内部 match 处理
/// - 架构状态 (status): 寄存器文件、CSR
pub struct CpuBuilder {
    entry_pc: u32,
    isa_config: IsaConfig,
    enable_f: bool,
    enable_d: bool,
    enable_v: bool,
    enable_m_mode: bool,
    enable_s_mode: bool,
}

impl CpuBuilder {
    /// 创建新的 CPU 构建器
    ///
    /// 默认启用 RV32I 基础指令集
    pub fn new(entry_pc: u32) -> Self {
        Self {
            entry_pc,
            isa_config: IsaConfig::new(),
            enable_f: false,
            enable_d: false,
            enable_v: false,
            enable_m_mode: true,  // M-mode 默认启用
            enable_s_mode: false,
        }
    }

    /// 启用 M 扩展（乘除法）
    pub fn with_m_extension(mut self) -> Self {
        self.isa_config = self.isa_config.with_m_extension();
        self
    }

    /// 启用 Zicsr 扩展（CSR 操作指令）
    pub fn with_zicsr_extension(mut self) -> Self {
        self.isa_config = self.isa_config.with_zicsr_extension();
        self
    }

    /// 启用特权指令扩展（MRET, SRET, WFI）
    pub fn with_priv_extension(mut self) -> Self {
        self.isa_config = self.isa_config.with_priv_extension();
        self
    }

    /// 启用 F 扩展（单精度浮点）
    pub fn with_f_extension(mut self) -> Self {
        self.enable_f = true;
        self.isa_config = self.isa_config.with_f_extension();
        self
    }

    /// 启用 D 扩展（双精度浮点，隐含 F）
    pub fn with_d_extension(mut self) -> Self {
        self.enable_f = true;
        self.enable_d = true;
        // TODO: self.isa_config = self.isa_config.with_d_extension();
        self
    }

    /// 启用 V 扩展（向量）
    pub fn with_v_extension(mut self) -> Self {
        self.enable_v = true;
        // TODO: self.isa_config = self.isa_config.with_v_extension();
        self
    }

    /// 启用 S-mode（监管者模式）
    pub fn with_s_mode(mut self) -> Self {
        self.enable_s_mode = true;
        self
    }

    /// 禁用 M-mode CSR（仅用于用户态模拟）
    pub fn without_m_mode(mut self) -> Self {
        self.enable_m_mode = false;
        self
    }

    /// 检测配置中的指令冲突
    pub fn detect_conflicts(&self) -> Vec<ConflictInfo> {
        self.isa_config.detect_conflicts()
    }

    /// 获取启用的扩展列表摘要
    pub fn extensions_summary(&self) -> String {
        let mut parts = vec!["RV32I".to_string()];
        
        // 从 isa_config 获取扩展
        // 这里简化处理，直接根据 builder 状态生成
        if self.isa_config.detect_conflicts().is_empty() {
            // M extension 通过 isa_config 管理
        }
        if self.enable_f {
            parts.push("F".to_string());
        }
        if self.enable_d {
            parts.push("D".to_string());
        }
        if self.enable_v {
            parts.push("V".to_string());
        }
        
        parts.join("")
    }

    /// 构建 CPU 核心
    ///
    /// 返回 `Err` 如果检测到指令冲突
    pub fn build(self) -> Result<CpuCore, Vec<ConflictInfo>> {
        // 1. 检测指令冲突
        let conflicts = self.isa_config.detect_conflicts();
        if !conflicts.is_empty() {
            return Err(conflicts);
        }

        // 2. 构建解码器
        let decoder = Arc::new(self.isa_config.build()?);

        // 3. 构建架构状态
        let mut status = Status::new();
        
        // 注册基础 CSR
        status.csr.register(csr_def::BASE_CSRS);

        // 根据扩展配置状态
        if self.enable_f || self.enable_d {
            status.enable_fp();
            status.csr.register(csr_def::F_CSRS);
        }

        if self.enable_v {
            status.enable_vec();
            status.csr.register(csr_def::V_CSRS);
        }

        // 特权级 CSR
        if self.enable_m_mode {
            status.csr.register(csr_def::M_CSRS);
        }

        if self.enable_s_mode {
            status.csr.register(csr_def::S_CSRS);
        }

        // 4. 创建 CPU 核心
        Ok(CpuCore::with_config(self.entry_pc, status, decoder))
    }
}

impl Default for CpuBuilder {
    fn default() -> Self {
        Self::new(0)
    }
}
