//! 解码器框架
//!
//! 提供可扩展的指令解码系统

use crate::isa::{DecodedInstr, RvInstr};
use std::sync::Arc;

/// 指令解码器 trait
/// 
/// 实现此 trait 以创建自定义解码器
pub trait InstrDecoder: Send + Sync {
    /// 解码器名称
    fn name(&self) -> &str;
    
    /// 尝试解码指令
    /// 
    /// 返回 `Some(decoded)` 如果能解码，否则返回 `None`
    fn decode(&self, raw: u32) -> Option<DecodedInstr>;
    
    /// 此解码器处理的 opcode 列表
    ///
    /// 用于优化：注册表可以只对特定 opcode 调用相应解码器
    fn handled_opcodes(&self) -> Option<&[u32]> {
        None
    }

        /// 是否允许与其他解码器在同一 opcode 上共存
        fn allow_opcode_overlap(&self) -> bool {
            false
        }
}

    /// 解码器注册表
    /// 
    /// 管理多个解码器，支持运行时注册和按优先级解码
pub struct DecoderRegistry {
    /// 注册的解码器列表（按注册顺序）
    decoders: Vec<Arc<dyn InstrDecoder>>,
    /// 按 opcode 分桶的解码器索引
    opcode_map: [Vec<usize>; 128],
}

impl DecoderRegistry {
    /// 创建空的解码器注册表
    pub fn new() -> Self {
        Self {
            decoders: Vec::new(),
            opcode_map: std::array::from_fn(|_| Vec::new()),
        }
    }
    
    /// 创建包含默认 RV32I 解码器的注册表
    pub fn with_rv32i() -> Self {
        let mut registry: DecoderRegistry = Self::new();
        let _ = registry.register(Arc::new(super::rv32i::RV32I_DECODER));
        registry
    }
    
    /// 注册一个解码器；若声明的 opcode 已被占用则返回 Err
    pub fn register(&mut self, decoder: Arc<dyn InstrDecoder>) -> Result<(), String> {
        let idx = self.decoders.len();

        // 先做冲突检测，避免错误时污染注册表
        if let Some(opcodes) = decoder.handled_opcodes() {
            for &op in opcodes {
                if (op as usize) < 128 {
                    if !self.opcode_map[op as usize].is_empty() {
                        let existing_conflict = self.opcode_map[op as usize]
                            .iter()
                            .any(|&i| !self.decoders[i].allow_opcode_overlap());
                        if existing_conflict || !decoder.allow_opcode_overlap() {
                            return Err(format!(
                                "opcode 0x{:02X} already handled; rejecting decoder {}",
                                op,
                                decoder.name()
                            ));
                        }
                    }
                }
            }
        } else {
            // 处理全 opcode 覆盖的解码器：任意已存在且不允许重叠则拒绝
            let has_blocking = self
                .opcode_map
                .iter()
                .any(|bucket| bucket.iter().any(|&i| !self.decoders[i].allow_opcode_overlap()));
            if has_blocking || !decoder.allow_opcode_overlap() {
                return Err(format!("wildcard decoder {} cannot register due to overlap", decoder.name()));
            }
        }

        // 冲突检测通过后再写入结构
        self.decoders.push(decoder);

        if let Some(opcodes) = self.decoders[idx].handled_opcodes() {
            for &op in opcodes {
                if (op as usize) < 128 {
                    self.opcode_map[op as usize].push(idx);
                }
            }
        } else {
            for bucket in &mut self.opcode_map {
                bucket.push(idx);
            }
        }

        Ok(())
    }
    
    /// 解码指令
    ///
    /// 仅按 opcode 分桶的解码器尝试，命中即返回
    pub fn decode(&self, raw: u32) -> DecodedInstr {
        let opcode = raw & 0x7F;

        // 按 opcode 分桶解码
        for &idx in &self.opcode_map[opcode as usize] {
            let decoder = &self.decoders[idx];
            if let Some(decoded) = decoder.decode(raw) {
                return decoded;
            }
        }

        DecodedInstr {
            raw,
            instr: RvInstr::Illegal { raw },
        }
    }
    
    /// 获取已注册的解码器数量
    pub fn decoder_count(&self) -> usize {
        self.decoders.len()
    }
    
    /// 列出所有已注册的解码器名称
    pub fn decoder_names(&self) -> Vec<&str> {
        self.decoders.iter().map(|d| d.name()).collect()
    }
}

impl Default for DecoderRegistry {
    fn default() -> Self {
        Self::with_rv32i()
    }
}

impl std::fmt::Debug for DecoderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecoderRegistry")
            .field("decoders", &self.decoder_names())
            .finish()
    }
}
