//! CSR definitions for each RISC-V extension.
//!
//! Each extension provides a static array of CsrEntry for registration.
//! 
//! 设计原则：
//! - 每个 CSR 先定义地址常量 `CSR_XXX`
//! - CsrEntry 使用这些常量，避免重复硬编码地址
//! - 常量用于代码中快速引用（如 trap 处理）
//! - CsrEntry 用于 CSR 注册和管理

use super::status::CsrEntry;

// ============================================================================
// Base Unprivileged CSR Addresses
// ============================================================================

pub const CSR_CYCLE: u16 = 0xC00;
pub const CSR_TIME: u16 = 0xC01;
pub const CSR_INSTRET: u16 = 0xC02;
pub const CSR_CYCLEH: u16 = 0xC80;
pub const CSR_TIMEH: u16 = 0xC81;
pub const CSR_INSTRETH: u16 = 0xC82;

/// Unprivileged counter/timer CSRs.
#[allow(dead_code)]
pub const BASE_CSRS: &[CsrEntry] = &[
    CsrEntry { name: "cycle",    addr: CSR_CYCLE,    reset: 0 },
    CsrEntry { name: "time",     addr: CSR_TIME,     reset: 0 },
    CsrEntry { name: "instret",  addr: CSR_INSTRET,  reset: 0 },
    CsrEntry { name: "cycleh",   addr: CSR_CYCLEH,   reset: 0 },
    CsrEntry { name: "timeh",    addr: CSR_TIMEH,    reset: 0 },
    CsrEntry { name: "instreth", addr: CSR_INSTRETH, reset: 0 },
];

// ============================================================================
// F/D Extension CSR Addresses (Floating-point)
// ============================================================================

pub const CSR_FFLAGS: u16 = 0x001;
pub const CSR_FRM: u16 = 0x002;
pub const CSR_FCSR: u16 = 0x003;

/// Floating-point CSRs for F/D extensions.
#[allow(dead_code)]
pub const F_CSRS: &[CsrEntry] = &[
    CsrEntry { name: "fflags", addr: CSR_FFLAGS, reset: 0 },
    CsrEntry { name: "frm",    addr: CSR_FRM,    reset: 0 },
    CsrEntry { name: "fcsr",   addr: CSR_FCSR,   reset: 0 },
];

// ============================================================================
// V Extension CSR Addresses (Vector)
// ============================================================================

pub const CSR_VSTART: u16 = 0x008;
pub const CSR_VXSAT: u16 = 0x009;
pub const CSR_VXRM: u16 = 0x00A;
pub const CSR_VCSR: u16 = 0x00F;
pub const CSR_VL: u16 = 0xC20;
pub const CSR_VTYPE: u16 = 0xC21;
pub const CSR_VLENB: u16 = 0xC22;

/// Vector CSRs for V extension.
#[allow(dead_code)]
pub const V_CSRS: &[CsrEntry] = &[
    CsrEntry { name: "vstart", addr: CSR_VSTART, reset: 0 },
    CsrEntry { name: "vxsat",  addr: CSR_VXSAT,  reset: 0 },
    CsrEntry { name: "vxrm",   addr: CSR_VXRM,   reset: 0 },
    CsrEntry { name: "vcsr",   addr: CSR_VCSR,   reset: 0 },
    CsrEntry { name: "vl",     addr: CSR_VL,     reset: 0 },
    CsrEntry { name: "vtype",  addr: CSR_VTYPE,  reset: 0 },
    CsrEntry { name: "vlenb",  addr: CSR_VLENB,  reset: 16 }, // VLEN/8, default VLEN=128
];

// ============================================================================
// Machine-level CSR Addresses (M-mode)
// ============================================================================

// Machine Information
pub const CSR_MVENDORID: u16 = 0xF11;
pub const CSR_MARCHID: u16 = 0xF12;
pub const CSR_MIMPID: u16 = 0xF13;
pub const CSR_MHARTID: u16 = 0xF14;
pub const CSR_MCONFIGPTR: u16 = 0xF15;

// Machine Trap Setup
pub const CSR_MSTATUS: u16 = 0x300;
pub const CSR_MISA: u16 = 0x301;
pub const CSR_MEDELEG: u16 = 0x302;
pub const CSR_MIDELEG: u16 = 0x303;
pub const CSR_MIE: u16 = 0x304;
pub const CSR_MTVEC: u16 = 0x305;
pub const CSR_MCOUNTEREN: u16 = 0x306;
pub const CSR_MSTATUSH: u16 = 0x310;

// Machine Trap Handling
pub const CSR_MSCRATCH: u16 = 0x340;
pub const CSR_MEPC: u16 = 0x341;
pub const CSR_MCAUSE: u16 = 0x342;
pub const CSR_MTVAL: u16 = 0x343;
pub const CSR_MIP: u16 = 0x344;
pub const CSR_MTINST: u16 = 0x34A;
pub const CSR_MTVAL2: u16 = 0x34B;

/// Machine-level CSRs.
#[allow(dead_code)]
pub const M_CSRS: &[CsrEntry] = &[
    // Machine Information
    CsrEntry { name: "mvendorid",  addr: CSR_MVENDORID,  reset: 0 },
    CsrEntry { name: "marchid",    addr: CSR_MARCHID,    reset: 0 },
    CsrEntry { name: "mimpid",     addr: CSR_MIMPID,     reset: 0 },
    CsrEntry { name: "mhartid",    addr: CSR_MHARTID,    reset: 0 },
    CsrEntry { name: "mconfigptr", addr: CSR_MCONFIGPTR, reset: 0 },
    // Machine Trap Setup
    CsrEntry { name: "mstatus",    addr: CSR_MSTATUS,    reset: 0 },
    CsrEntry { name: "misa",       addr: CSR_MISA,       reset: 0 },
    CsrEntry { name: "medeleg",    addr: CSR_MEDELEG,    reset: 0 },
    CsrEntry { name: "mideleg",    addr: CSR_MIDELEG,    reset: 0 },
    CsrEntry { name: "mie",        addr: CSR_MIE,        reset: 0 },
    CsrEntry { name: "mtvec",      addr: CSR_MTVEC,      reset: 0 },
    CsrEntry { name: "mcounteren", addr: CSR_MCOUNTEREN, reset: 0 },
    CsrEntry { name: "mstatush",   addr: CSR_MSTATUSH,   reset: 0 },
    // Machine Trap Handling
    CsrEntry { name: "mscratch",   addr: CSR_MSCRATCH,   reset: 0 },
    CsrEntry { name: "mepc",       addr: CSR_MEPC,       reset: 0 },
    CsrEntry { name: "mcause",     addr: CSR_MCAUSE,     reset: 0 },
    CsrEntry { name: "mtval",      addr: CSR_MTVAL,      reset: 0 },
    CsrEntry { name: "mip",        addr: CSR_MIP,        reset: 0 },
    CsrEntry { name: "mtinst",     addr: CSR_MTINST,     reset: 0 },
    CsrEntry { name: "mtval2",     addr: CSR_MTVAL2,     reset: 0 },
];

// ============================================================================
// Supervisor-level CSR Addresses (S-mode)
// ============================================================================

// Supervisor Trap Setup
pub const CSR_SSTATUS: u16 = 0x100;
pub const CSR_SIE: u16 = 0x104;
pub const CSR_STVEC: u16 = 0x105;
pub const CSR_SCOUNTEREN: u16 = 0x106;

// Supervisor Trap Handling
pub const CSR_SSCRATCH: u16 = 0x140;
pub const CSR_SEPC: u16 = 0x141;
pub const CSR_SCAUSE: u16 = 0x142;
pub const CSR_STVAL: u16 = 0x143;
pub const CSR_SIP: u16 = 0x144;

// Supervisor Address Translation
pub const CSR_SATP: u16 = 0x180;

/// Supervisor-level CSRs.
#[allow(dead_code)]
pub const S_CSRS: &[CsrEntry] = &[
    // Supervisor Trap Setup
    CsrEntry { name: "sstatus",    addr: CSR_SSTATUS,    reset: 0 },
    CsrEntry { name: "sie",        addr: CSR_SIE,        reset: 0 },
    CsrEntry { name: "stvec",      addr: CSR_STVEC,      reset: 0 },
    CsrEntry { name: "scounteren", addr: CSR_SCOUNTEREN, reset: 0 },
    // Supervisor Trap Handling
    CsrEntry { name: "sscratch",   addr: CSR_SSCRATCH,   reset: 0 },
    CsrEntry { name: "sepc",       addr: CSR_SEPC,       reset: 0 },
    CsrEntry { name: "scause",     addr: CSR_SCAUSE,     reset: 0 },
    CsrEntry { name: "stval",      addr: CSR_STVAL,      reset: 0 },
    CsrEntry { name: "sip",        addr: CSR_SIP,        reset: 0 },
    // Supervisor Address Translation
    CsrEntry { name: "satp",       addr: CSR_SATP,       reset: 0 },
];
