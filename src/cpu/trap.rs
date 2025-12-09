//! Trap (异常和中断) 处理机制
//!
//! 本模块定义 RISC-V 特权架构中的 trap 相关类型和处理逻辑。
//!
//! # Trap 类别
//!
//! - **异常 (Exception)**: 同步事件，由当前指令触发
//! - **中断 (Interrupt)**: 异步事件，由外部或定时器触发
//!
//! # 特权级
//!
//! - **Machine (M)**: 最高特权级，必须实现
//! - **Supervisor (S)**: 可选的监管者模式
//! - **User (U)**: 用户模式

/// 特权级模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PrivilegeMode {
    /// User mode
    User = 0,
    /// Supervisor mode
    Supervisor = 1,
    /// Reserved
    _Reserved = 2,
    /// Machine mode (default, highest privilege)
    #[default]
    Machine = 3,
}

impl PrivilegeMode {
    /// 从 2-bit 编码创建特权级
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0x3 {
            0 => PrivilegeMode::User,
            1 => PrivilegeMode::Supervisor,
            3 => PrivilegeMode::Machine,
            _ => PrivilegeMode::Machine, // Reserved -> fallback to Machine
        }
    }

    /// 转换为 2-bit 编码
    pub fn to_bits(self) -> u8 {
        self as u8
    }
}

/// Trap 原因 (异常或中断)
///
/// 编码遵循 RISC-V 特权规范的 mcause/scause 格式：
/// - 最高位 (bit 31 for RV32) = 1 表示中断，= 0 表示异常
/// - 低位为具体原因码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapCause {
    // ========== 异常 (同步) ==========
    /// 指令地址未对齐 (code = 0)
    InstructionAddressMisaligned,
    /// 指令访问错误 (code = 1)
    InstructionAccessFault,
    /// 非法指令 (code = 2)
    IllegalInstruction,
    /// 断点 (code = 3)
    Breakpoint,
    /// 加载地址未对齐 (code = 4)
    LoadAddressMisaligned,
    /// 加载访问错误 (code = 5)
    LoadAccessFault,
    /// 存储地址未对齐 (code = 6)
    StoreAddressMisaligned,
    /// 存储访问错误 (code = 7)
    StoreAccessFault,
    /// 来自 U-mode 的环境调用 (code = 8)
    EcallFromU,
    /// 来自 S-mode 的环境调用 (code = 9)
    EcallFromS,
    // code 10 reserved
    /// 来自 M-mode 的环境调用 (code = 11)
    EcallFromM,
    /// 指令页错误 (code = 12)
    InstructionPageFault,
    /// 加载页错误 (code = 13)
    LoadPageFault,
    // code 14 reserved
    /// 存储页错误 (code = 15)
    StorePageFault,

    // ========== 中断 (异步) ==========
    /// 用户软件中断 (code = 0, interrupt)
    UserSoftwareInterrupt,
    /// 监管者软件中断 (code = 1, interrupt)
    SupervisorSoftwareInterrupt,
    // code 2 reserved
    /// 机器软件中断 (code = 3, interrupt)
    MachineSoftwareInterrupt,
    /// 用户定时器中断 (code = 4, interrupt)
    UserTimerInterrupt,
    /// 监管者定时器中断 (code = 5, interrupt)
    SupervisorTimerInterrupt,
    // code 6 reserved
    /// 机器定时器中断 (code = 7, interrupt)
    MachineTimerInterrupt,
    /// 用户外部中断 (code = 8, interrupt)
    UserExternalInterrupt,
    /// 监管者外部中断 (code = 9, interrupt)
    SupervisorExternalInterrupt,
    // code 10 reserved
    /// 机器外部中断 (code = 11, interrupt)
    MachineExternalInterrupt,
}

impl TrapCause {
    /// 是否为中断（异步事件）
    pub fn is_interrupt(&self) -> bool {
        matches!(
            self,
            TrapCause::UserSoftwareInterrupt
                | TrapCause::SupervisorSoftwareInterrupt
                | TrapCause::MachineSoftwareInterrupt
                | TrapCause::UserTimerInterrupt
                | TrapCause::SupervisorTimerInterrupt
                | TrapCause::MachineTimerInterrupt
                | TrapCause::UserExternalInterrupt
                | TrapCause::SupervisorExternalInterrupt
                | TrapCause::MachineExternalInterrupt
        )
    }

    /// 是否为异常（同步事件）
    pub fn is_exception(&self) -> bool {
        !self.is_interrupt()
    }

    /// 获取异常/中断代码（mcause 的低位）
    pub fn code(&self) -> u32 {
        match self {
            // Exceptions
            TrapCause::InstructionAddressMisaligned => 0,
            TrapCause::InstructionAccessFault => 1,
            TrapCause::IllegalInstruction => 2,
            TrapCause::Breakpoint => 3,
            TrapCause::LoadAddressMisaligned => 4,
            TrapCause::LoadAccessFault => 5,
            TrapCause::StoreAddressMisaligned => 6,
            TrapCause::StoreAccessFault => 7,
            TrapCause::EcallFromU => 8,
            TrapCause::EcallFromS => 9,
            TrapCause::EcallFromM => 11,
            TrapCause::InstructionPageFault => 12,
            TrapCause::LoadPageFault => 13,
            TrapCause::StorePageFault => 15,
            // Interrupts
            TrapCause::UserSoftwareInterrupt => 0,
            TrapCause::SupervisorSoftwareInterrupt => 1,
            TrapCause::MachineSoftwareInterrupt => 3,
            TrapCause::UserTimerInterrupt => 4,
            TrapCause::SupervisorTimerInterrupt => 5,
            TrapCause::MachineTimerInterrupt => 7,
            TrapCause::UserExternalInterrupt => 8,
            TrapCause::SupervisorExternalInterrupt => 9,
            TrapCause::MachineExternalInterrupt => 11,
        }
    }

    /// 转换为 mcause/scause 寄存器值
    ///
    /// 对于 RV32: bit 31 = interrupt, bits 30:0 = code
    pub fn to_cause_value(&self) -> u32 {
        let interrupt_bit = if self.is_interrupt() { 1u32 << 31 } else { 0 };
        interrupt_bit | self.code()
    }

    /// 根据当前特权级获取对应的 ECALL 异常
    pub fn ecall_from(mode: PrivilegeMode) -> Self {
        match mode {
            PrivilegeMode::User => TrapCause::EcallFromU,
            PrivilegeMode::Supervisor => TrapCause::EcallFromS,
            PrivilegeMode::Machine => TrapCause::EcallFromM,
            PrivilegeMode::_Reserved => TrapCause::EcallFromM,
        }
    }
}

// ========== mstatus 字段位置常量 ==========

/// mstatus 寄存器字段
pub mod mstatus {
    // 字段位置
    pub const UIE: u32 = 0;      // User Interrupt Enable
    pub const SIE: u32 = 1;      // Supervisor Interrupt Enable
    pub const MIE: u32 = 3;      // Machine Interrupt Enable
    pub const UPIE: u32 = 4;     // User Previous Interrupt Enable
    pub const SPIE: u32 = 5;     // Supervisor Previous Interrupt Enable
    pub const MPIE: u32 = 7;     // Machine Previous Interrupt Enable
    pub const SPP: u32 = 8;      // Supervisor Previous Privilege (1 bit)
    pub const MPP: u32 = 11;     // Machine Previous Privilege (2 bits)
    pub const FS: u32 = 13;      // FPU State (2 bits)
    pub const XS: u32 = 15;      // Extension State (2 bits)
    pub const MPRV: u32 = 17;    // Modify PRiVilege
    pub const SUM: u32 = 18;     // Supervisor User Memory access
    pub const MXR: u32 = 19;     // Make eXecutable Readable
    pub const TVM: u32 = 20;     // Trap Virtual Memory
    pub const TW: u32 = 21;      // Timeout Wait
    pub const TSR: u32 = 22;     // Trap SRET
    pub const SD: u32 = 31;      // State Dirty

    // 字段掩码
    pub const MIE_MASK: u32 = 1 << MIE;
    pub const MPIE_MASK: u32 = 1 << MPIE;
    pub const MPP_MASK: u32 = 0x3 << MPP;
    pub const SIE_MASK: u32 = 1 << SIE;
    pub const SPIE_MASK: u32 = 1 << SPIE;
    pub const SPP_MASK: u32 = 1 << SPP;

    /// 从 mstatus 值读取 MPP 字段
    #[inline]
    pub fn read_mpp(mstatus: u32) -> u8 {
        ((mstatus >> MPP) & 0x3) as u8
    }

    /// 向 mstatus 值写入 MPP 字段
    #[inline]
    pub fn write_mpp(mstatus: u32, mpp: u8) -> u32 {
        (mstatus & !MPP_MASK) | (((mpp & 0x3) as u32) << MPP)
    }

    /// 从 mstatus 值读取 MIE 字段
    #[inline]
    pub fn read_mie(mstatus: u32) -> bool {
        (mstatus & MIE_MASK) != 0
    }

    /// 从 mstatus 值读取 MPIE 字段
    #[inline]
    pub fn read_mpie(mstatus: u32) -> bool {
        (mstatus & MPIE_MASK) != 0
    }
}

// ========== mtvec 模式 ==========

/// mtvec 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TvecMode {
    /// Direct: 所有 trap 跳转到 BASE
    Direct = 0,
    /// Vectored: 异常跳转到 BASE，中断跳转到 BASE + 4*cause
    Vectored = 1,
}

impl TvecMode {
    pub fn from_bits(bits: u32) -> Self {
        match bits & 0x3 {
            0 => TvecMode::Direct,
            1 => TvecMode::Vectored,
            _ => TvecMode::Direct, // Reserved -> Direct
        }
    }
}

/// 解析 mtvec/stvec 寄存器，返回 (base, mode)
#[inline]
pub fn parse_tvec(tvec: u32) -> (u32, TvecMode) {
    let mode: TvecMode = TvecMode::from_bits(tvec);
    let base = tvec & !0x3; // 低 2 位是 mode，其余是 base
    (base, mode)
}

/// 计算 trap handler 地址
#[inline]
pub fn calculate_trap_pc(tvec: u32, cause: &TrapCause) -> u32 {
    let (base, mode) = parse_tvec(tvec);
    match mode {
        TvecMode::Direct => base,
        TvecMode::Vectored => {
            if cause.is_interrupt() {
                base.wrapping_add(4 * cause.code())
            } else {
                base
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trap_cause_encoding() {
        assert_eq!(TrapCause::IllegalInstruction.to_cause_value(), 2);
        assert_eq!(TrapCause::Breakpoint.to_cause_value(), 3);
        assert_eq!(TrapCause::EcallFromM.to_cause_value(), 11);
        
        // 中断应该设置最高位
        assert_eq!(TrapCause::MachineTimerInterrupt.to_cause_value(), 0x80000007);
        assert_eq!(TrapCause::MachineExternalInterrupt.to_cause_value(), 0x8000000B);
    }

    #[test]
    fn test_privilege_mode() {
        assert_eq!(PrivilegeMode::User.to_bits(), 0);
        assert_eq!(PrivilegeMode::Supervisor.to_bits(), 1);
        assert_eq!(PrivilegeMode::Machine.to_bits(), 3);
        
        assert_eq!(PrivilegeMode::from_bits(0), PrivilegeMode::User);
        assert_eq!(PrivilegeMode::from_bits(1), PrivilegeMode::Supervisor);
        assert_eq!(PrivilegeMode::from_bits(3), PrivilegeMode::Machine);
    }

    #[test]
    fn test_ecall_from_mode() {
        assert_eq!(TrapCause::ecall_from(PrivilegeMode::User), TrapCause::EcallFromU);
        assert_eq!(TrapCause::ecall_from(PrivilegeMode::Supervisor), TrapCause::EcallFromS);
        assert_eq!(TrapCause::ecall_from(PrivilegeMode::Machine), TrapCause::EcallFromM);
    }

    #[test]
    fn test_mstatus_fields() {
        let mstatus = 0x00001888; // MPP=3, MPIE=1, MIE=1
        assert_eq!(mstatus::read_mpp(mstatus), 3);
        assert!(mstatus::read_mpie(mstatus));
        assert!(mstatus::read_mie(mstatus));
        
        let mstatus2 = mstatus::write_mpp(mstatus, 1);
        assert_eq!(mstatus::read_mpp(mstatus2), 1);
    }

    #[test]
    fn test_tvec_parsing() {
        // Direct mode, base = 0x80000000
        let tvec = 0x80000000;
        let (base, mode) = parse_tvec(tvec);
        assert_eq!(base, 0x80000000);
        assert_eq!(mode, TvecMode::Direct);

        // Vectored mode, base = 0x80000000
        let tvec = 0x80000001;
        let (base, mode) = parse_tvec(tvec);
        assert_eq!(base, 0x80000000);
        assert_eq!(mode, TvecMode::Vectored);
    }

    #[test]
    fn test_trap_pc_calculation() {
        // Direct mode
        let tvec = 0x80000000;
        assert_eq!(calculate_trap_pc(tvec, &TrapCause::IllegalInstruction), 0x80000000);
        assert_eq!(calculate_trap_pc(tvec, &TrapCause::MachineTimerInterrupt), 0x80000000);

        // Vectored mode: exceptions go to base, interrupts go to base + 4*cause
        let tvec = 0x80000001;
        assert_eq!(calculate_trap_pc(tvec, &TrapCause::IllegalInstruction), 0x80000000);
        // Machine timer interrupt (code=7) -> base + 4*7 = base + 28
        assert_eq!(calculate_trap_pc(tvec, &TrapCause::MachineTimerInterrupt), 0x8000001C);
    }
}
