//! 定义指令的语义表达式，用于解码和执行阶段

use std::fmt;

/// 自定义指令的 trait
/// 
/// 用于支持非标准 RISC-V 扩展或完全自定义的指令集
pub trait CustomInstr: fmt::Debug + Send + Sync {
    /// 获取指令名称
    fn name(&self) -> &str;
    
    /// 克隆为 Box
    fn clone_box(&self) -> Box<dyn CustomInstr>;
    
    /// 比较相等
    fn eq_custom(&self, other: &dyn CustomInstr) -> bool;
}

/// RV32I 指令的语义化表示
///
/// 使用枚举来表示已解码的指令，每个变体包含该指令所需的所有操作数。
/// 这种表示方式的优点：
/// - 解码阶段一次性做完字段提取与符号扩展
/// - 执行阶段代码清晰、分支少，有利于维护和优化
/// - 添加新指令只需扩展枚举与匹配逻辑，不影响整体结构
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RvInstr {
    // ========== R-type 算术/逻辑指令 ==========
    /// ADD: rd = rs1 + rs2
    Add { rd: u8, rs1: u8, rs2: u8 },
    /// SUB: rd = rs1 - rs2
    Sub { rd: u8, rs1: u8, rs2: u8 },
    /// AND: rd = rs1 & rs2
    And { rd: u8, rs1: u8, rs2: u8 },
    /// OR: rd = rs1 | rs2
    Or { rd: u8, rs1: u8, rs2: u8 },
    /// XOR: rd = rs1 ^ rs2
    Xor { rd: u8, rs1: u8, rs2: u8 },
    /// SLT: rd = (rs1 < rs2) ? 1 : 0 (有符号比较)
    Slt { rd: u8, rs1: u8, rs2: u8 },
    /// SLTU: rd = (rs1 < rs2) ? 1 : 0 (无符号比较)
    Sltu { rd: u8, rs1: u8, rs2: u8 },
    /// SLL: rd = rs1 << rs2[4:0]
    Sll { rd: u8, rs1: u8, rs2: u8 },
    /// SRL: rd = rs1 >> rs2[4:0] (逻辑右移)
    Srl { rd: u8, rs1: u8, rs2: u8 },
    /// SRA: rd = rs1 >> rs2[4:0] (算术右移)
    Sra { rd: u8, rs1: u8, rs2: u8 },

    // ========== I-type 立即数算术/逻辑指令 ==========
    /// ADDI: rd = rs1 + imm
    Addi { rd: u8, rs1: u8, imm: i32 },
    /// ANDI: rd = rs1 & imm
    Andi { rd: u8, rs1: u8, imm: i32 },
    /// ORI: rd = rs1 | imm
    Ori { rd: u8, rs1: u8, imm: i32 },
    /// XORI: rd = rs1 ^ imm
    Xori { rd: u8, rs1: u8, imm: i32 },
    /// SLTI: rd = (rs1 < imm) ? 1 : 0 (有符号比较)
    Slti { rd: u8, rs1: u8, imm: i32 },
    /// SLTIU: rd = (rs1 < imm) ? 1 : 0 (无符号比较)
    Sltiu { rd: u8, rs1: u8, imm: i32 },
    /// SLLI: rd = rs1 << shamt
    Slli { rd: u8, rs1: u8, shamt: u8 },
    /// SRLI: rd = rs1 >> shamt (逻辑右移)
    Srli { rd: u8, rs1: u8, shamt: u8 },
    /// SRAI: rd = rs1 >> shamt (算术右移)
    Srai { rd: u8, rs1: u8, shamt: u8 },

    // ========== Load 指令 ==========
    /// LB: rd = sign_extend(mem[rs1 + offset])
    Lb { rd: u8, rs1: u8, offset: i32 },
    /// LH: rd = sign_extend(mem[rs1 + offset])
    Lh { rd: u8, rs1: u8, offset: i32 },
    /// LW: rd = mem[rs1 + offset]
    Lw { rd: u8, rs1: u8, offset: i32 },
    /// LBU: rd = zero_extend(mem[rs1 + offset])
    Lbu { rd: u8, rs1: u8, offset: i32 },
    /// LHU: rd = zero_extend(mem[rs1 + offset])
    Lhu { rd: u8, rs1: u8, offset: i32 },

    // ========== Store 指令 ==========
    /// SB: mem[rs1 + offset] = rs2[7:0]
    Sb { rs1: u8, rs2: u8, offset: i32 },
    /// SH: mem[rs1 + offset] = rs2[15:0]
    Sh { rs1: u8, rs2: u8, offset: i32 },
    /// SW: mem[rs1 + offset] = rs2
    Sw { rs1: u8, rs2: u8, offset: i32 },

    // ========== U-type 指令 ==========
    /// LUI: rd = imm << 12
    Lui { rd: u8, imm: i32 },
    /// AUIPC: rd = pc + (imm << 12)
    Auipc { rd: u8, imm: i32 },

    // ========== 控制流指令 ==========
    /// JAL: rd = pc + 4; pc = pc + offset
    Jal { rd: u8, offset: i32 },
    /// JALR: rd = pc + 4; pc = (rs1 + offset) & !1
    Jalr { rd: u8, rs1: u8, offset: i32 },
    /// BEQ: if (rs1 == rs2) pc = pc + offset
    Beq { rs1: u8, rs2: u8, offset: i32 },
    /// BNE: if (rs1 != rs2) pc = pc + offset
    Bne { rs1: u8, rs2: u8, offset: i32 },
    /// BLT: if (rs1 < rs2) pc = pc + offset (有符号)
    Blt { rs1: u8, rs2: u8, offset: i32 },
    /// BGE: if (rs1 >= rs2) pc = pc + offset (有符号)
    Bge { rs1: u8, rs2: u8, offset: i32 },
    /// BLTU: if (rs1 < rs2) pc = pc + offset (无符号)
    Bltu { rs1: u8, rs2: u8, offset: i32 },
    /// BGEU: if (rs1 >= rs2) pc = pc + offset (无符号)
    Bgeu { rs1: u8, rs2: u8, offset: i32 },

    // ========== 系统指令 ==========
    /// ECALL: 环境调用
    Ecall,
    /// EBREAK: 断点
    Ebreak,
    /// FENCE: 内存顺序栅栏，pred/succ/fm 表征访问域
    Fence { pred: u8, succ: u8, fm: u8 },
    /// FENCE.I: 指令取指栅栏（Zifencei）
    FenceI,

    // ========== M 扩展（乘除法）==========
    /// MUL: rd = (rs1 * rs2)[31:0]
    Mul { rd: u8, rs1: u8, rs2: u8 },
    /// MULH: rd = (rs1 * rs2)[63:32] (signed * signed)
    Mulh { rd: u8, rs1: u8, rs2: u8 },
    /// MULHSU: rd = (rs1 * rs2)[63:32] (signed * unsigned)
    Mulhsu { rd: u8, rs1: u8, rs2: u8 },
    /// MULHU: rd = (rs1 * rs2)[63:32] (unsigned * unsigned)
    Mulhu { rd: u8, rs1: u8, rs2: u8 },
    /// DIV: rd = rs1 / rs2 (signed)
    Div { rd: u8, rs1: u8, rs2: u8 },
    /// DIVU: rd = rs1 / rs2 (unsigned)
    Divu { rd: u8, rs1: u8, rs2: u8 },
    /// REM: rd = rs1 % rs2 (signed)
    Rem { rd: u8, rs1: u8, rs2: u8 },
    /// REMU: rd = rs1 % rs2 (unsigned)
    Remu { rd: u8, rs1: u8, rs2: u8 },

    // ========== Zicsr 扩展（CSR 操作）==========
    /// CSRRW: t = CSR[csr]; CSR[csr] = rs1; rd = t
    /// 原子读写 CSR
    Csrrw { rd: u8, rs1: u8, csr: u16 },
    /// CSRRS: t = CSR[csr]; CSR[csr] = t | rs1; rd = t
    /// 读 CSR 并置位（rs1 中为 1 的位置位）
    Csrrs { rd: u8, rs1: u8, csr: u16 },
    /// CSRRC: t = CSR[csr]; CSR[csr] = t & ~rs1; rd = t
    /// 读 CSR 并清位（rs1 中为 1 的位清零）
    Csrrc { rd: u8, rs1: u8, csr: u16 },
    /// CSRRWI: t = CSR[csr]; CSR[csr] = zimm; rd = t
    /// 立即数版本的原子读写
    Csrrwi { rd: u8, zimm: u8, csr: u16 },
    /// CSRRSI: t = CSR[csr]; CSR[csr] = t | zimm; rd = t
    /// 立即数版本的读并置位
    Csrrsi { rd: u8, zimm: u8, csr: u16 },
    /// CSRRCI: t = CSR[csr]; CSR[csr] = t & ~zimm; rd = t
    /// 立即数版本的读并清位
    Csrrci { rd: u8, zimm: u8, csr: u16 },

    // ========== 特权指令 ==========
    /// MRET: 从 M-mode trap handler 返回
    /// 
    /// 执行流程：
    /// 1. 将 mstatus.MPIE 恢复到 mstatus.MIE
    /// 2. 将特权级设置为 mstatus.MPP
    /// 3. 将 mstatus.MPP 设置为 U (或 M，如果不支持 U)
    /// 4. 将 mstatus.MPIE 设置为 1
    /// 5. PC = mepc
    Mret,
    
    /// SRET: 从 S-mode trap handler 返回
    /// 
    /// 类似 MRET，但操作 sstatus.SPIE/SPP 和 sepc
    Sret,
    
    /// WFI: 等待中断
    /// 
    /// 暂停执行直到有中断发生
    Wfi,

    // ========== F 扩展（单精度浮点）==========
    /// FLW: 从内存加载单精度浮点数
    /// frd = M[rs1 + offset]
    Flw { frd: u8, rs1: u8, offset: i32 },
    /// FSW: 存储单精度浮点数到内存
    /// M[rs1 + offset] = frs2
    Fsw { frs2: u8, rs1: u8, offset: i32 },

    /// FADD.S: 单精度浮点加法
    FaddS { frd: u8, frs1: u8, frs2: u8, rm: u8 },
    /// FSUB.S: 单精度浮点减法
    FsubS { frd: u8, frs1: u8, frs2: u8, rm: u8 },
    /// FMUL.S: 单精度浮点乘法
    FmulS { frd: u8, frs1: u8, frs2: u8, rm: u8 },
    /// FDIV.S: 单精度浮点除法
    FdivS { frd: u8, frs1: u8, frs2: u8, rm: u8 },
    /// FSQRT.S: 单精度浮点平方根
    FsqrtS { frd: u8, frs1: u8, rm: u8 },

    /// FMADD.S: 融合乘加 frd = frs1 * frs2 + frs3
    FmaddS { frd: u8, frs1: u8, frs2: u8, frs3: u8, rm: u8 },
    /// FMSUB.S: 融合乘减 frd = frs1 * frs2 - frs3
    FmsubS { frd: u8, frs1: u8, frs2: u8, frs3: u8, rm: u8 },
    /// FNMADD.S: 负融合乘加 frd = -(frs1 * frs2) - frs3
    FnmaddS { frd: u8, frs1: u8, frs2: u8, frs3: u8, rm: u8 },
    /// FNMSUB.S: 负融合乘减 frd = -(frs1 * frs2) + frs3
    FnmsubS { frd: u8, frs1: u8, frs2: u8, frs3: u8, rm: u8 },

    /// FSGNJ.S: 符号注入（取 frs2 的符号）
    FsgnjS { frd: u8, frs1: u8, frs2: u8 },
    /// FSGNJN.S: 符号注入（取 frs2 符号的反）
    FsgnjnS { frd: u8, frs1: u8, frs2: u8 },
    /// FSGNJX.S: 符号注入（符号异或）
    FsgnjxS { frd: u8, frs1: u8, frs2: u8 },

    /// FMIN.S: 取最小值
    FminS { frd: u8, frs1: u8, frs2: u8 },
    /// FMAX.S: 取最大值
    FmaxS { frd: u8, frs1: u8, frs2: u8 },

    /// FEQ.S: 浮点相等比较，结果写入整数寄存器
    FeqS { rd: u8, frs1: u8, frs2: u8 },
    /// FLT.S: 浮点小于比较
    FltS { rd: u8, frs1: u8, frs2: u8 },
    /// FLE.S: 浮点小于等于比较
    FleS { rd: u8, frs1: u8, frs2: u8 },

    /// FCVT.W.S: 浮点转有符号整数
    FcvtWS { rd: u8, frs1: u8, rm: u8 },
    /// FCVT.WU.S: 浮点转无符号整数
    FcvtWuS { rd: u8, frs1: u8, rm: u8 },
    /// FCVT.S.W: 有符号整数转浮点
    FcvtSW { frd: u8, rs1: u8, rm: u8 },
    /// FCVT.S.WU: 无符号整数转浮点
    FcvtSWu { frd: u8, rs1: u8, rm: u8 },

    /// FMV.X.W: 浮点寄存器位模式移动到整数寄存器
    FmvXW { rd: u8, frs1: u8 },
    /// FMV.W.X: 整数寄存器位模式移动到浮点寄存器
    FmvWX { frd: u8, rs1: u8 },
    /// FCLASS.S: 浮点分类
    FclassS { rd: u8, frs1: u8 },

    // ========== 特殊 ==========
    /// 非法指令
    Illegal { raw: u32 },
    
    /// 自定义扩展指令
    /// 
    /// 用于支持非标准扩展或实验性指令
    Custom {
        /// 扩展标识符（如 "vendor_x", "gpgpu" 等）
        extension: &'static str,
        /// 操作码
        opcode: u8,
        /// 原始编码
        raw: u32,
        /// 解码后的字段（根据扩展自定义）
        fields: CustomFields,
    },
}

/// 自定义指令的字段
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub struct CustomFields {
    pub rd: Option<u8>,
    pub rs1: Option<u8>,
    pub rs2: Option<u8>,
    pub rs3: Option<u8>,
    pub imm: Option<i32>,
    /// 扩展特定的额外数据
    pub extra: u64,
}


impl CustomFields {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_rd(mut self, rd: u8) -> Self {
        self.rd = Some(rd);
        self
    }
    
    pub fn with_rs1(mut self, rs1: u8) -> Self {
        self.rs1 = Some(rs1);
        self
    }
    
    pub fn with_rs2(mut self, rs2: u8) -> Self {
        self.rs2 = Some(rs2);
        self
    }
    
    pub fn with_imm(mut self, imm: i32) -> Self {
        self.imm = Some(imm);
        self
    }
    
    pub fn with_extra(mut self, extra: u64) -> Self {
        self.extra = extra;
        self
    }
}

impl Copy for CustomFields {}

// RvInstr 现在可以 Copy，因为 CustomFields 也是 Copy
impl Copy for RvInstr {}

/// 已解码的指令
///
/// 包含原始编码与解码后的语义信息
#[derive(Debug, Clone)]
pub struct DecodedInstr {
    /// 原始 32-bit 指令编码
    pub raw: u32,
    /// 解码后的语义表示
    pub instr: RvInstr,
}

impl Copy for DecodedInstr {}
