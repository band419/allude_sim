//! RV32F (single-precision floating-point) execution unit
//!
//! 实现 RISC-V F 扩展的所有指令

use super::super::{CpuCore, MemAccessType};
use crate::isa::RvInstr;
use crate::memory::Memory;
use simple_soft_float::{F32, FPState, RoundingMode, StatusFlags};

/// FCSR 地址
const FCSR_ADDR: u16 = 0x003;
/// FFLAGS 地址
#[allow(dead_code)]
const FFLAGS_ADDR: u16 = 0x001;
/// FRM 地址
#[allow(dead_code)]
const FRM_ADDR: u16 = 0x002;

/// 浮点异常标志位
#[allow(dead_code)]
mod fflags {
    pub const NX: u32 = 1 << 0;  // 不精确
    pub const UF: u32 = 1 << 1;  // 下溢
    pub const OF: u32 = 1 << 2;  // 上溢
    pub const DZ: u32 = 1 << 3;  // 除以零
    pub const NV: u32 = 1 << 4;  // 无效操作
}

/// 设置浮点异常标志
#[inline]
fn set_fflags(cpu: &mut CpuCore, flags: u32) {
    let fcsr = cpu.csr_read(FCSR_ADDR);
    cpu.csr_write(FCSR_ADDR, fcsr | flags);
}

#[inline]
fn decode_rounding_mode(cpu: &CpuCore, instr_rm: u8) -> Option<RoundingMode> {
    let rm = if instr_rm == 0b111 {
        ((cpu.csr_read(FCSR_ADDR) >> 5) & 0b111) as u8
    } else {
        instr_rm
    };

    match rm {
        0b000 => Some(RoundingMode::TiesToEven),
        0b001 => Some(RoundingMode::TowardZero),
        0b010 => Some(RoundingMode::TowardNegative),
        0b011 => Some(RoundingMode::TowardPositive),
        0b100 => Some(RoundingMode::TiesToAway),
        _ => None,
    }
}

#[inline]
fn apply_fp_state(cpu: &mut CpuCore, fp_state: &FPState) {
    let flags = fp_state.status_flags;
    let mut bits = 0;
    if flags.contains(StatusFlags::INVALID_OPERATION) {
        bits |= fflags::NV;
    }
    if flags.contains(StatusFlags::DIVISION_BY_ZERO) {
        bits |= fflags::DZ;
    }
    if flags.contains(StatusFlags::OVERFLOW) {
        bits |= fflags::OF;
    }
    if flags.contains(StatusFlags::UNDERFLOW) {
        bits |= fflags::UF;
    }
    if flags.contains(StatusFlags::INEXACT) {
        bits |= fflags::NX;
    }

    if bits != 0 {
        set_fflags(cpu, bits);
    }
}

#[inline]
fn read_soft(cpu: &CpuCore, reg: u8) -> F32 {
    F32::from_bits(cpu.read_fp(reg))
}

#[inline]
fn write_soft(cpu: &mut CpuCore, reg: u8, value: F32) {
    cpu.write_fp(reg, value.into_bits());
}

#[inline]
fn is_signaling_nan_bits(bits: u32) -> bool {
    let exp = bits & 0x7F80_0000;
    let frac = bits & 0x007F_FFFF;
    exp == 0x7F80_0000 && frac != 0 && (frac & 0x0040_0000) == 0
}

fn handle_min_max(cpu: &mut CpuCore, frd: u8, frs1: u8, frs2: u8, is_min: bool) {
    let a_bits = cpu.read_fp(frs1);
    let b_bits = cpu.read_fp(frs2);
    let a = f32::from_bits(a_bits);
    let b = f32::from_bits(b_bits);

    let a_nan = a.is_nan();
    let b_nan = b.is_nan();
    let mut flag_bits = 0;
    if is_signaling_nan_bits(a_bits) || is_signaling_nan_bits(b_bits) {
        flag_bits |= fflags::NV;
    }

    let result_bits = if a_nan && b_nan {
        CANONICAL_NAN
    } else if a_nan {
        b_bits
    } else if b_nan {
        a_bits
    } else if a == 0.0 && b == 0.0 {
        // For min, return -0 if either is -0; for max return +0 if either is +0
        if is_min {
            // min(+0, -0) = -0
            a_bits | b_bits  // OR gives -0 if either is -0
        } else {
            // max(+0, -0) = +0
            a_bits & b_bits  // AND gives +0 if either is +0
        }
    } else if a_bits == b_bits {
        a_bits
    } else {
        let choose_a = if is_min { a < b } else { a > b };
        if choose_a { a_bits } else { b_bits }
    };

    cpu.write_fp(frd, result_bits);

    if flag_bits != 0 {
        set_fflags(cpu, flag_bits);
    }
}

/// 浮点分类
fn fclass(value: f32) -> u32 {
    let bits = value.to_bits();
    let sign = bits >> 31;
    let exp = (bits >> 23) & 0xFF;
    let frac = bits & 0x007FFFFF;

    if exp == 0xFF {
        if frac == 0 {
            // Infinity
            if sign == 0 { 1 << 7 } else { 1 << 0 }  // +inf : -inf
        } else if frac & 0x00400000 != 0 {
            // Quiet NaN
            1 << 9
        } else {
            // Signaling NaN
            1 << 8
        }
    } else if exp == 0 {
        if frac == 0 {
            // Zero
            if sign == 0 { 1 << 4 } else { 1 << 3 }  // +0 : -0
        } else {
            // Subnormal
            if sign == 0 { 1 << 5 } else { 1 << 2 }  // +subnormal : -subnormal
        }
    } else {
        // Normal
        if sign == 0 { 1 << 6 } else { 1 << 1 }  // +normal : -normal
    }
}

/// 规范 NaN（Canonical NaN）
const CANONICAL_NAN: u32 = 0x7FC00000;

/// Execute RV32F (single-precision floating-point) instructions.
/// Returns true if handled.
pub fn execute(cpu: &mut CpuCore, mem: &mut dyn Memory, instr: RvInstr, current_pc: u32) -> bool {
    // 检查是否启用了浮点扩展
    if !cpu.has_fp() {
        return false;
    }

    match instr {
        // ========== Load/Store ==========
        RvInstr::Flw { frd, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            if let Some(value) = cpu.mem_result(mem.load32(addr), MemAccessType::Load, current_pc) {
                cpu.write_fp(frd, value);
            } else {
                return true;
            }
        }

        RvInstr::Fsw { frs2, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = cpu.read_fp(frs2);
            if !cpu.mem_result_unit(mem.store32(addr, value), MemAccessType::Store, current_pc) {
                return true;
            }
        }

        // ========== Arithmetic ==========
        RvInstr::FaddS { frd, frs1, frs2, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let mut fp_state = FPState::default();
            let result = a.add(&b, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FsubS { frd, frs1, frs2, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let mut fp_state = FPState::default();
            let result = a.sub(&b, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FmulS { frd, frs1, frs2, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let mut fp_state = FPState::default();
            let result = a.mul(&b, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FdivS { frd, frs1, frs2, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let mut fp_state = FPState::default();
            let result = a.div(&b, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FsqrtS { frd, frs1, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let mut fp_state = FPState::default();
            let result = a.sqrt(Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        // ========== Fused Multiply-Add ==========
        RvInstr::FmaddS { frd, frs1, frs2, frs3, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let c = read_soft(cpu, frs3);
            let mut fp_state = FPState::default();
            // fmadd: a * b + c
            let result = a.fused_mul_add(&b, &c, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FmsubS { frd, frs1, frs2, frs3, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let c = read_soft(cpu, frs3);
            let mut fp_state = FPState::default();
            // fmsub: a * b - c = a * b + (-c)
            let mut neg_c = c.clone();
            neg_c.toggle_sign();
            let result = a.fused_mul_add(&b, &neg_c, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FnmaddS { frd, frs1, frs2, frs3, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let c = read_soft(cpu, frs3);
            let mut fp_state = FPState::default();
            // fnmadd: -(a * b) - c = (-a) * b + (-c)
            let mut neg_a = a.clone();
            neg_a.toggle_sign();
            let mut neg_c = c.clone();
            neg_c.toggle_sign();
            let result = neg_a.fused_mul_add(&b, &neg_c, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FnmsubS { frd, frs1, frs2, frs3, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let c = read_soft(cpu, frs3);
            let mut fp_state = FPState::default();
            // fnmsub: -(a * b) + c = (-a) * b + c
            let mut neg_a = a.clone();
            neg_a.toggle_sign();
            let result = neg_a.fused_mul_add(&b, &c, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        // ========== Sign Injection ==========
        RvInstr::FsgnjS { frd, frs1, frs2 } => {
            let a = cpu.read_fp(frs1);
            let b = cpu.read_fp(frs2);
            // 取 frs1 的绝对值，frs2 的符号
            let result = (a & 0x7FFFFFFF) | (b & 0x80000000);
            cpu.write_fp(frd, result);
        }

        RvInstr::FsgnjnS { frd, frs1, frs2 } => {
            let a = cpu.read_fp(frs1);
            let b = cpu.read_fp(frs2);
            // 取 frs1 的绝对值，frs2 符号取反
            let result = (a & 0x7FFFFFFF) | ((b ^ 0x80000000) & 0x80000000);
            cpu.write_fp(frd, result);
        }

        RvInstr::FsgnjxS { frd, frs1, frs2 } => {
            let a = cpu.read_fp(frs1);
            let b = cpu.read_fp(frs2);
            // 取 frs1 的值，符号位异或
            let result = a ^ (b & 0x80000000);
            cpu.write_fp(frd, result);
        }

        // ========== Min/Max ==========
        RvInstr::FminS { frd, frs1, frs2 } => {
            handle_min_max(cpu, frd, frs1, frs2, true);
        }

        RvInstr::FmaxS { frd, frs1, frs2 } => {
            handle_min_max(cpu, frd, frs1, frs2, false);
        }

        // ========== Compare ==========
        RvInstr::FeqS { rd, frs1, frs2 } => {
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let mut fp_state = FPState::default();
            // compare_quiet doesn't signal for quiet NaN
            let result = a.compare_quiet(&b, Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            cpu.write_reg(rd, if result == Some(std::cmp::Ordering::Equal) { 1 } else { 0 });
        }

        RvInstr::FltS { rd, frs1, frs2 } => {
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let mut fp_state = FPState::default();
            // compare_signaling signals for any NaN
            let result = a.compare_signaling(&b, Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            cpu.write_reg(rd, if result == Some(std::cmp::Ordering::Less) { 1 } else { 0 });
        }

        RvInstr::FleS { rd, frs1, frs2 } => {
            let a = read_soft(cpu, frs1);
            let b = read_soft(cpu, frs2);
            let mut fp_state = FPState::default();
            // compare_signaling signals for any NaN
            let result = a.compare_signaling(&b, Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            let is_le = matches!(result, Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal));
            cpu.write_reg(rd, if is_le { 1 } else { 0 });
        }

        // ========== Classification ==========
        RvInstr::FclassS { rd, frs1 } => {
            let value = cpu.read_fp_f32(frs1);
            cpu.write_reg(rd, fclass(value));
        }

        // ========== Conversion: Float -> Integer ==========
        RvInstr::FcvtWS { rd, frs1, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let value = read_soft(cpu, frs1);
            let bits = value.into_bits();
            let mut fp_state = FPState::default();
            // exact=true 使得在结果不精确时设置 INEXACT 标志
            let result = value.to_i32(true, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            // On overflow/invalid, return saturated value per RISC-V spec
            let int_result = result.unwrap_or_else(|| {
                // Check if value is NaN
                let exp = (bits >> 23) & 0xFF;
                let frac = bits & 0x007FFFFF;
                let is_nan = exp == 0xFF && frac != 0;
                let is_neg_inf = bits == 0xFF800000; // -inf
                
                if is_nan {
                    // NaN -> INT_MAX
                    i32::MAX
                } else if is_neg_inf || (bits & 0x80000000) != 0 {
                    // -inf or negative overflow -> INT_MIN
                    i32::MIN
                } else {
                    // +inf or positive overflow -> INT_MAX
                    i32::MAX
                }
            });
            cpu.write_reg(rd, int_result as u32);
        }

        RvInstr::FcvtWuS { rd, frs1, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let value = read_soft(cpu, frs1);
            let bits = value.into_bits();
            let mut fp_state = FPState::default();
            // exact=true 使得在结果不精确时设置 INEXACT 标志
            let result = value.to_u32(true, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            // On overflow/invalid, return saturated value per RISC-V spec
            let int_result = result.unwrap_or_else(|| {
                // Check if value is NaN
                let exp = (bits >> 23) & 0xFF;
                let frac = bits & 0x007FFFFF;
                let is_nan = exp == 0xFF && frac != 0;
                let is_neg = (bits & 0x80000000) != 0;
                
                if is_nan || !is_neg {
                    // NaN or positive overflow -> UINT_MAX
                    u32::MAX
                } else {
                    // Negative value -> 0
                    0u32
                }
            });
            cpu.write_reg(rd, int_result);
        }

        // ========== Conversion: Integer -> Float ==========
        RvInstr::FcvtSW { frd, rs1, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let value = cpu.read_reg(rs1) as i32;
            let mut fp_state = FPState::default();
            let result = F32::from_i32(value, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        RvInstr::FcvtSWu { frd, rs1, rm } => {
            let Some(rounding) = decode_rounding_mode(cpu, rm) else { return false; };
            let value = cpu.read_reg(rs1);
            let mut fp_state = FPState::default();
            let result = F32::from_u32(value, Some(rounding), Some(&mut fp_state));
            apply_fp_state(cpu, &fp_state);
            write_soft(cpu, frd, result);
        }

        // ========== Move ==========
        RvInstr::FmvXW { rd, frs1 } => {
            // 从浮点寄存器移动到整数寄存器（位模式不变）
            let value = cpu.read_fp(frs1);
            cpu.write_reg(rd, value);
        }

        RvInstr::FmvWX { frd, rs1 } => {
            // 从整数寄存器移动到浮点寄存器（位模式不变）
            let value = cpu.read_reg(rs1);
            cpu.write_fp(frd, value);
        }

        _ => return false,
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::CpuBuilder;
    use crate::memory::FlatMemory;

    fn setup_fp_cpu() -> CpuCore {
        CpuBuilder::new(0x1000)
            .with_f_extension()
            .build()
            .expect("Failed to build CPU")
    }

    fn exec(cpu: &mut CpuCore, mem: &mut FlatMemory, instr: RvInstr) {
        let pc = cpu.pc();
        let _ = super::execute(cpu, mem, instr, pc);
    }

    #[test]
    fn test_fmv_w_x_and_fmv_x_w() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_reg(1, 0x40490FDB);
        let instr = RvInstr::FmvWX { frd: 1, rs1: 1 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_fp(1), 0x40490FDB);

        let instr = RvInstr::FmvXW { rd: 2, frs1: 1 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 0x40490FDB);
    }

    #[test]
    fn test_fadd_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 1.0);
        cpu.write_fp_f32(2, 2.0);

        let instr = RvInstr::FaddS { frd: 3, frs1: 1, frs2: 2, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(3);
        assert!((result - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fsub_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 5.0);
        cpu.write_fp_f32(2, 3.0);

        let instr = RvInstr::FsubS { frd: 3, frs1: 1, frs2: 2, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(3);
        assert!((result - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fmul_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 3.0);
        cpu.write_fp_f32(2, 4.0);

        let instr = RvInstr::FmulS { frd: 3, frs1: 1, frs2: 2, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(3);
        assert!((result - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fdiv_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 10.0);
        cpu.write_fp_f32(2, 2.0);

        let instr = RvInstr::FdivS { frd: 3, frs1: 1, frs2: 2, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(3);
        assert!((result - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fsqrt_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 16.0);

        let instr = RvInstr::FsqrtS { frd: 2, frs1: 1, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(2);
        assert!((result - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fmadd_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 2.0);
        cpu.write_fp_f32(2, 3.0);
        cpu.write_fp_f32(3, 1.0);

        let instr = RvInstr::FmaddS { frd: 4, frs1: 1, frs2: 2, frs3: 3, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(4);
        assert!((result - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fcvt_w_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 42.7);

        let instr = RvInstr::FcvtWS { rd: 2, frs1: 1, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        assert_eq!(cpu.read_reg(2), 43);
    }

    #[test]
    fn test_fcvt_s_w() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_reg(1, 42);

        let instr = RvInstr::FcvtSW { frd: 1, rs1: 1, rm: 0 };
        exec(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(1);
        assert!((result - 42.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fclass_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 1.0);
        let instr = RvInstr::FclassS { rd: 2, frs1: 1 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 1 << 6);

        cpu.write_fp_f32(1, f32::INFINITY);
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 1 << 7);

        cpu.write_fp_f32(1, f32::NEG_INFINITY);
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 1 << 0);
    }

    #[test]
    fn test_flw_fsw() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        let pi_bits: u32 = std::f32::consts::PI.to_bits();
        mem.store32(0x100, pi_bits).unwrap();

        cpu.write_reg(1, 0x100);
        let instr = RvInstr::Flw { frd: 1, rs1: 1, offset: 0 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_fp(1), pi_bits);

        cpu.write_reg(2, 0x200);
        let instr = RvInstr::Fsw { rs1: 2, frs2: 1, offset: 0 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(mem.load32(0x200).unwrap(), pi_bits);
    }

    #[test]
    fn test_feq_flt_fle() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 1.0);
        cpu.write_fp_f32(2, 2.0);
        cpu.write_fp_f32(3, 1.0);

        let instr = RvInstr::FeqS { rd: 10, frs1: 1, frs2: 2 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 0);

        let instr = RvInstr::FeqS { rd: 10, frs1: 1, frs2: 3 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 1);

        let instr = RvInstr::FltS { rd: 10, frs1: 1, frs2: 2 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 1);

        let instr = RvInstr::FleS { rd: 10, frs1: 1, frs2: 3 };
        exec(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 1);
    }

    #[test]
    fn test_fsgnj() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 3.0);
        cpu.write_fp_f32(2, -5.0);

        let instr = RvInstr::FsgnjS { frd: 3, frs1: 1, frs2: 2 };
        exec(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - (-3.0)).abs() < f32::EPSILON);

        let instr = RvInstr::FsgnjnS { frd: 3, frs1: 1, frs2: 2 };
        exec(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - 3.0).abs() < f32::EPSILON);

        let instr = RvInstr::FsgnjxS { frd: 3, frs1: 1, frs2: 2 };
        exec(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - (-3.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fmin_fmax() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 1.0);
        cpu.write_fp_f32(2, 5.0);

        let instr = RvInstr::FminS { frd: 3, frs1: 1, frs2: 2 };
        exec(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - 1.0).abs() < f32::EPSILON);

        let instr = RvInstr::FmaxS { frd: 3, frs1: 1, frs2: 2 };
        exec(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - 5.0).abs() < f32::EPSILON);
    }
}
