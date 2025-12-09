//! ISA 配置与冲突检测
//!
//! 提供指令集扩展的配置管理和冲突检测机制

use std::collections::HashSet;
use std::sync::Arc;

use super::decoder::{DecoderRegistry, InstrDecoder};
use super::instr_def::InstrDef;
use super::rv32i::{RV32I_DECODER, RV32I_INSTRS};
use super::rv32m::{RV32M_DECODER, RV32M_INSTRS};
use super::rv32f::{RV32F_DECODER, RV32F_INSTRS};
use super::zicsr::{ZICSR_DECODER, ZICSR_INSTRS};
use super::priv_instr::{PRIV_DECODER, PRIV_INSTRS};

/// 支持的 ISA 扩展
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IsaExtension {
    /// RV32I 基础整数指令集（必选）
    RV32I,
    /// M 扩展：乘除法
    RV32M,
    /// A 扩展：原子操作（未实现）
    RV32A,
    /// F 扩展：单精度浮点（未实现）
    RV32F,
    /// D 扩展：双精度浮点（未实现）
    RV32D,
    /// C 扩展：压缩指令（未实现）
    RV32C,
    /// Zicsr 扩展：CSR 操作指令
    Zicsr,
    /// 特权指令：MRET, SRET, WFI 等
    Priv,
    /// 自定义扩展
    Custom(&'static str),
}

impl std::fmt::Display for IsaExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsaExtension::RV32I => write!(f, "RV32I"),
            IsaExtension::RV32M => write!(f, "M"),
            IsaExtension::RV32A => write!(f, "A"),
            IsaExtension::RV32F => write!(f, "F"),
            IsaExtension::RV32D => write!(f, "D"),
            IsaExtension::RV32C => write!(f, "C"),
            IsaExtension::Zicsr => write!(f, "_Zicsr"),
            IsaExtension::Priv => write!(f, "_Priv"),
            IsaExtension::Custom(name) => write!(f, "X{}", name),
        }
    }
}

/// 指令模式描述（用于冲突检测）
/// 
/// 现在这是一个轻量级视图，可以从 InstrDef 派生
#[derive(Debug, Clone)]
pub struct InstrSignature {
    /// 扩展名称
    pub extension: IsaExtension,
    /// 指令名称
    pub name: &'static str,
    /// 匹配掩码
    pub mask: u32,
    /// 匹配值
    pub match_val: u32,
}

impl InstrSignature {
    pub const fn new(
        extension: IsaExtension,
        name: &'static str,
        mask: u32,
        match_val: u32,
    ) -> Self {
        Self {
            extension,
            name,
            mask,
            match_val,
        }
    }

    /// 从 InstrDef 创建签名
    pub fn from_def(def: &InstrDef, extension: IsaExtension) -> Self {
        Self {
            extension,
            name: def.name,
            mask: def.mask,
            match_val: def.match_val,
        }
    }

    /// 检查两个指令模式是否冲突
    pub fn conflicts_with(&self, other: &InstrSignature) -> bool {
        // 两个模式冲突当且仅当存在某个指令字同时匹配两者
        // 即：(mask1 & mask2 & match1) == (mask1 & mask2 & match2)
        let common_mask = self.mask & other.mask;
        (self.match_val & common_mask) == (other.match_val & common_mask)
    }
}

/// 冲突信息
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub instr1: InstrSignature,
    pub instr2: InstrSignature,
    /// 冲突的示例指令编码
    pub example_raw: u32,
}

impl std::fmt::Display for ConflictInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "冲突: {}:{} 与 {}:{} (示例: 0x{:08X})",
            self.instr1.extension,
            self.instr1.name,
            self.instr2.extension,
            self.instr2.name,
            self.example_raw
        )
    }
}

/// ISA 配置构建器
/// 
/// 用于配置启用的指令集扩展，检测冲突，并构建解码器
/// 
/// # 示例
/// 
/// ```
/// use allude_sim::isa::IsaConfig;
/// 
/// let config = IsaConfig::new()
///     .with_m_extension()
///     .build()
///     .expect("无冲突");
/// ```
pub struct IsaConfig {
    extensions: HashSet<IsaExtension>,
    custom_decoders: Vec<(IsaExtension, Arc<dyn InstrDecoder>, Vec<InstrSignature>)>,
    signatures: Vec<InstrSignature>,
}

impl IsaConfig {
    /// 创建新的 ISA 配置（默认只有 RV32I）
    pub fn new() -> Self {
        let mut config = Self {
            extensions: HashSet::new(),
            custom_decoders: Vec::new(),
            signatures: Vec::new(),
        };
        
        // RV32I 是必选的
        config.extensions.insert(IsaExtension::RV32I);
        config.signatures.extend(rv32i_signatures());
        
        config
    }

    /// 启用 M 扩展（乘除法）
    pub fn with_m_extension(mut self) -> Self {
        if self.extensions.insert(IsaExtension::RV32M) {
            self.signatures.extend(rv32m_signatures());
        }
        self
    }

    /// 启用 F 扩展（单精度浮点）
    pub fn with_f_extension(mut self) -> Self {
        if self.extensions.insert(IsaExtension::RV32F) {
            self.signatures.extend(rv32f_signatures());
        }
        self
    }

    /// 启用 Zicsr 扩展（CSR 操作指令）
    pub fn with_zicsr_extension(mut self) -> Self {
        if self.extensions.insert(IsaExtension::Zicsr) {
            self.signatures.extend(zicsr_signatures());
        }
        self
    }

    /// 启用特权指令扩展（MRET, SRET, WFI）
    pub fn with_priv_extension(mut self) -> Self {
        if self.extensions.insert(IsaExtension::Priv) {
            self.signatures.extend(priv_signatures());
        }
        self
    }

    /// 添加自定义解码器
    /// 
    /// # 参数
    /// 
    /// * `extension` - 扩展标识
    /// * `decoder` - 解码器实现
    /// * `signatures` - 该解码器处理的指令签名（用于冲突检测）
    pub fn with_custom_decoder(
        mut self,
        extension: IsaExtension,
        decoder: Arc<dyn InstrDecoder>,
        signatures: Vec<InstrSignature>,
    ) -> Self {
        self.extensions.insert(extension);
        self.signatures.extend(signatures.clone());
        self.custom_decoders.push((extension, decoder, signatures));
        self
    }

    /// 检测指令冲突
    pub fn detect_conflicts(&self) -> Vec<ConflictInfo> {
        let mut conflicts = Vec::new();
        
        for (i, sig1) in self.signatures.iter().enumerate() {
            for sig2 in self.signatures.iter().skip(i + 1) {
                // 同一扩展内的指令不检测（假设扩展内部是正确的）
                if sig1.extension == sig2.extension {
                    continue;
                }
                
                if sig1.conflicts_with(sig2) {
                    // 生成一个同时满足两者 mask/match 的示例
                    let example = (sig1.match_val & sig1.mask) | (sig2.match_val & sig2.mask);
                    conflicts.push(ConflictInfo {
                        instr1: sig1.clone(),
                        instr2: sig2.clone(),
                        example_raw: example,
                    });
                }
            }
        }
        
        conflicts
    }

    /// 检查配置是否有效（无冲突）
    pub fn is_valid(&self) -> bool {
        self.detect_conflicts().is_empty()
    }

    /// 获取 ISA 字符串（如 "RV32IM"）
    pub fn isa_string(&self) -> String {
        let mut s = String::from("RV32");
        
        // 按标准顺序添加扩展
        let order = [
            IsaExtension::RV32M,
            IsaExtension::RV32A,
            IsaExtension::RV32F,
            IsaExtension::RV32D,
            IsaExtension::RV32C,
        ];
        
        for ext in &order {
            if self.extensions.contains(ext) {
                s.push_str(&ext.to_string());
            }
        }
        
        // 添加自定义扩展（排序以保证输出稳定）
        let mut custom: Vec<&str> = self
            .extensions
            .iter()
            .filter_map(|ext| match ext {
                IsaExtension::Custom(name) => Some(*name),
                _ => None,
            })
            .collect();
        custom.sort_unstable();
        for name in custom {
            s.push_str(&format!("_X{}", name));
        }
        
        s
    }

    /// 构建解码器注册表
    /// 
    /// 如果存在冲突，返回错误
    pub fn build(self) -> Result<DecoderRegistry, Vec<ConflictInfo>> {
        let conflicts = self.detect_conflicts();
        if !conflicts.is_empty() {
            return Err(conflicts);
        }
        
        let mut registry = DecoderRegistry::new();
        
        // 添加基础 RV32I
        registry.register(Arc::new(RV32I_DECODER)).expect("RV32I decoder must register");
        
        // 添加 M 扩展
        if self.extensions.contains(&IsaExtension::RV32M) {
            registry
                .register(Arc::new(RV32M_DECODER))
                .expect("RV32M decoder must register");
        }
        
        // 添加 F 扩展
        if self.extensions.contains(&IsaExtension::RV32F) {
            registry
                .register(Arc::new(RV32F_DECODER))
                .expect("RV32F decoder must register");
        }
        
        // 添加 Zicsr 扩展
        if self.extensions.contains(&IsaExtension::Zicsr) {
            registry
                .register(Arc::new(ZICSR_DECODER))
                .expect("Zicsr decoder must register");
        }
        
        // 添加特权指令扩展
        if self.extensions.contains(&IsaExtension::Priv) {
            registry
                .register(Arc::new(PRIV_DECODER))
                .expect("Priv decoder must register");
        }
        
        // 添加自定义解码器
        for (_, decoder, _) in self.custom_decoders {
            registry.register(decoder).expect("custom decoder registration failed");
        }
        
        Ok(registry)
    }

    /// 构建解码器，忽略冲突警告
    pub fn build_unchecked(self) -> DecoderRegistry {
        let mut registry = DecoderRegistry::new();
        
        let _ = registry.register(Arc::new(RV32I_DECODER));
        
        if self.extensions.contains(&IsaExtension::RV32M) {
            let _ = registry.register(Arc::new(RV32M_DECODER));
        }
        
        if self.extensions.contains(&IsaExtension::RV32F) {
            let _ = registry.register(Arc::new(RV32F_DECODER));
        }
        
        if self.extensions.contains(&IsaExtension::Zicsr) {
            let _ = registry.register(Arc::new(ZICSR_DECODER));
        }
        
        for (_, decoder, _) in self.custom_decoders {
            let _ = registry.register(decoder);
        }
        
        registry
    }

    /// 获取已启用的扩展列表
    pub fn enabled_extensions(&self) -> &HashSet<IsaExtension> {
        &self.extensions
    }

    /// 打印配置摘要
    pub fn summary(&self) -> String {
        let mut s = format!("ISA: {}\n", self.isa_string());
        s.push_str(&format!("扩展: {:?}\n", self.extensions));
        s.push_str(&format!("指令签名数: {}\n", self.signatures.len()));
        
        let conflicts = self.detect_conflicts();
        if conflicts.is_empty() {
            s.push_str("状态: ✓ 无冲突\n");
        } else {
            s.push_str(&format!("状态: ✗ {} 个冲突\n", conflicts.len()));
            for c in &conflicts {
                s.push_str(&format!("  - {}\n", c));
            }
        }
        
        s
    }
}

impl Default for IsaConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 从 InstrDef 派生的指令签名 ==========

/// RV32I 指令签名（从 RV32I_INSTRS 派生）
fn rv32i_signatures() -> Vec<InstrSignature> {
    RV32I_INSTRS
        .iter()
        .map(|def| InstrSignature::from_def(def, IsaExtension::RV32I))
        .collect()
}

/// RV32M 指令签名（从 RV32M_INSTRS 派生）
fn rv32m_signatures() -> Vec<InstrSignature> {
    RV32M_INSTRS
        .iter()
        .map(|def| InstrSignature::from_def(def, IsaExtension::RV32M))
        .collect()
}

/// RV32F 指令签名（从 RV32F_INSTRS 派生）
fn rv32f_signatures() -> Vec<InstrSignature> {
    RV32F_INSTRS
        .iter()
        .map(|def| InstrSignature::from_def(def, IsaExtension::RV32F))
        .collect()
}

/// Zicsr 指令签名（从 ZICSR_INSTRS 派生）
fn zicsr_signatures() -> Vec<InstrSignature> {
    ZICSR_INSTRS
        .iter()
        .map(|def| InstrSignature::from_def(def, IsaExtension::Zicsr))
        .collect()
}

/// 特权指令签名（从 PRIV_INSTRS 派生）
fn priv_signatures() -> Vec<InstrSignature> {
    PRIV_INSTRS
        .iter()
        .map(|def| InstrSignature::from_def(def, IsaExtension::Priv))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_config() {
        let config = IsaConfig::new();
        assert!(config.is_valid());
        assert_eq!(config.isa_string(), "RV32");
    }

    #[test]
    fn test_with_m_extension() {
        let config = IsaConfig::new().with_m_extension();
        assert!(config.is_valid());
        assert_eq!(config.isa_string(), "RV32M");
        
        let registry = config.build().unwrap();
        assert_eq!(registry.decoder_count(), 2);
    }

    #[test]
    fn test_no_conflict_between_rv32i_and_m() {
        let config = IsaConfig::new().with_m_extension();
        let conflicts = config.detect_conflicts();
        assert!(conflicts.is_empty(), "不应该有冲突: {:?}", conflicts);
    }

    #[test]
    fn test_conflict_detection() {
        // 创建一个故意冲突的签名
        let sig1 = InstrSignature::new(
            IsaExtension::Custom("test1"),
            "CONFLICT1",
            0x707F,  // 只检查 opcode + funct3
            0x0033,  // 与 ADD 的 opcode+funct3 相同
        );
        
        let sig2 = InstrSignature::new(
            IsaExtension::Custom("test2"),
            "CONFLICT2",
            0x707F,
            0x0033,
        );
        
        assert!(sig1.conflicts_with(&sig2));
    }

    #[test]
    fn test_no_conflict_different_funct7() {
        // ADD: funct7=0, funct3=0, op=0110011
        let add_sig = InstrSignature::new(
            IsaExtension::RV32I,
            "ADD",
            0xFE00707F,
            0x0033,
        );
        
        // MUL: funct7=1, funct3=0, op=0110011
        let mul_sig = InstrSignature::new(
            IsaExtension::RV32M,
            "MUL",
            0xFE00707F,
            0x02000033,
        );
        
        assert!(!add_sig.conflicts_with(&mul_sig));
    }

    #[test]
    fn test_summary() {
        let config = IsaConfig::new().with_m_extension();
        let summary = config.summary();
        assert!(summary.contains("RV32M"));
        assert!(summary.contains("无冲突"));
    }
}
