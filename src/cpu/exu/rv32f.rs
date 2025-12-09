//! RV32F (single-precision floating-point) execution unit
//!
//! 实现 RISC-V F 扩展的所有指令

use super::super::CpuCore;
use crate::isa::RvInstr;
use crate::memory::Memory;

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

/// 获取当前舍入模式
#[inline]
fn get_rounding_mode(cpu: &CpuCore, instr_rm: u8) -> std::num::FpCategory {
    let rm = if instr_rm == 0b111 {
        // 动态舍入：从 frm 读取
        ((cpu.csr_read(FCSR_ADDR) >> 5) & 0b111) as u8
    } else {
        instr_rm
    };
    // 返回舍入模式，这里暂时只返回一个占位符
    // 实际实现需要根据 rm 值设置不同的舍入行为
    let _ = rm;
    std::num::FpCategory::Normal
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

/// 检查并设置 NaN 标志
fn check_nan(cpu: &mut CpuCore, a: f32, b: f32) -> bool {
    if a.is_nan() || b.is_nan() {
        set_fflags(cpu, fflags::NV);
        true
    } else {
        false
    }
}

/// 规范 NaN（Canonical NaN）
const CANONICAL_NAN: u32 = 0x7FC00000;

/// Execute RV32F (single-precision floating-point) instructions.
/// Returns true if handled.
pub fn execute(cpu: &mut CpuCore, mem: &mut dyn Memory, instr: RvInstr) -> bool {
    // 检查是否启用了浮点扩展
    if !cpu.has_fp() {
        return false;
    }

    // 忽略舍入模式（暂时使用默认舍入）
    let _ = get_rounding_mode(cpu, 0);

    match instr {
        // ========== Load/Store ==========
        RvInstr::Flw { frd, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = mem.load32(addr);
            cpu.write_fp(frd, value);
        }

        RvInstr::Fsw { frs2, rs1, offset } => {
            let addr = cpu.read_reg(rs1).wrapping_add(offset as u32);
            let value = cpu.read_fp(frs2);
            mem.store32(addr, value);
        }

        // ========== Arithmetic ==========
        RvInstr::FaddS { frd, frs1, frs2, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let result = a + b;
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FsubS { frd, frs1, frs2, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let result = a - b;
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FmulS { frd, frs1, frs2, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let result = a * b;
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FdivS { frd, frs1, frs2, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            if b == 0.0 {
                set_fflags(cpu, fflags::DZ);
            }
            let result = a / b;
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FsqrtS { frd, frs1, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            if a < 0.0 {
                set_fflags(cpu, fflags::NV);
                cpu.write_fp(frd, CANONICAL_NAN);
            } else {
                let result = a.sqrt();
                cpu.write_fp_f32(frd, result);
            }
        }

        // ========== Fused Multiply-Add ==========
        RvInstr::FmaddS { frd, frs1, frs2, frs3, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let c = cpu.read_fp_f32(frs3);
            // (frs1 × frs2) + frs3
            let result = a.mul_add(b, c);
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FmsubS { frd, frs1, frs2, frs3, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let c = cpu.read_fp_f32(frs3);
            // (frs1 × frs2) - frs3
            let result = a.mul_add(b, -c);
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FnmaddS { frd, frs1, frs2, frs3, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let c = cpu.read_fp_f32(frs3);
            // -(frs1 × frs2) - frs3
            let result = (-a).mul_add(b, -c);
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FnmsubS { frd, frs1, frs2, frs3, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let c = cpu.read_fp_f32(frs3);
            // -(frs1 × frs2) + frs3
            let result = (-a).mul_add(b, c);
            cpu.write_fp_f32(frd, result);
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
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let result = if a.is_nan() && b.is_nan() {
                set_fflags(cpu, fflags::NV);
                f32::from_bits(CANONICAL_NAN)
            } else if a.is_nan() {
                b
            } else if b.is_nan() {
                a
            } else if a == 0.0 && b == 0.0 {
                // -0.0 < +0.0
                if a.to_bits() > b.to_bits() { a } else { b }
            } else {
                a.min(b)
            };
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FmaxS { frd, frs1, frs2 } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            let result = if a.is_nan() && b.is_nan() {
                set_fflags(cpu, fflags::NV);
                f32::from_bits(CANONICAL_NAN)
            } else if a.is_nan() {
                b
            } else if b.is_nan() {
                a
            } else if a == 0.0 && b == 0.0 {
                // +0.0 > -0.0
                if a.to_bits() < b.to_bits() { a } else { b }
            } else {
                a.max(b)
            };
            cpu.write_fp_f32(frd, result);
        }

        // ========== Compare ==========
        RvInstr::FeqS { rd, frs1, frs2 } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            // 只有 signaling NaN 才设置 NV 标志
            if a.is_nan() || b.is_nan() {
                // 检查是否为 signaling NaN
                let a_bits = a.to_bits();
                let b_bits = b.to_bits();
                let a_is_snan = a.is_nan() && (a_bits & 0x00400000 == 0);
                let b_is_snan = b.is_nan() && (b_bits & 0x00400000 == 0);
                if a_is_snan || b_is_snan {
                    set_fflags(cpu, fflags::NV);
                }
                cpu.write_reg(rd, 0);
            } else {
                cpu.write_reg(rd, if a == b { 1 } else { 0 });
            }
        }

        RvInstr::FltS { rd, frs1, frs2 } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            if check_nan(cpu, a, b) {
                cpu.write_reg(rd, 0);
            } else {
                cpu.write_reg(rd, if a < b { 1 } else { 0 });
            }
        }

        RvInstr::FleS { rd, frs1, frs2 } => {
            let a = cpu.read_fp_f32(frs1);
            let b = cpu.read_fp_f32(frs2);
            if check_nan(cpu, a, b) {
                cpu.write_reg(rd, 0);
            } else {
                cpu.write_reg(rd, if a <= b { 1 } else { 0 });
            }
        }

        // ========== Classification ==========
        RvInstr::FclassS { rd, frs1 } => {
            let value = cpu.read_fp_f32(frs1);
            cpu.write_reg(rd, fclass(value));
        }

        // ========== Conversion: Float -> Integer ==========
        RvInstr::FcvtWS { rd, frs1, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let result = if a.is_nan() {
                set_fflags(cpu, fflags::NV);
                i32::MAX as u32
            } else if a >= i32::MAX as f32 {
                set_fflags(cpu, fflags::NV);
                i32::MAX as u32
            } else if a <= i32::MIN as f32 {
                set_fflags(cpu, fflags::NV);
                i32::MIN as u32
            } else {
                a.round() as i32 as u32
            };
            cpu.write_reg(rd, result);
        }

        RvInstr::FcvtWuS { rd, frs1, rm: _ } => {
            let a = cpu.read_fp_f32(frs1);
            let result = if a.is_nan() || a < 0.0 {
                set_fflags(cpu, fflags::NV);
                if a.is_nan() || a >= 0.0 { u32::MAX } else { 0 }
            } else if a >= u32::MAX as f32 {
                set_fflags(cpu, fflags::NV);
                u32::MAX
            } else {
                a.round() as u32
            };
            cpu.write_reg(rd, result);
        }

        // ========== Conversion: Integer -> Float ==========
        RvInstr::FcvtSW { frd, rs1, rm: _ } => {
            let a = cpu.read_reg(rs1) as i32;
            let result = a as f32;
            cpu.write_fp_f32(frd, result);
        }

        RvInstr::FcvtSWu { frd, rs1, rm: _ } => {
            let a = cpu.read_reg(rs1);
            let result = a as f32;
            cpu.write_fp_f32(frd, result);
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
            .expect("配置无冲突")
    }

    #[test]
    fn test_fmv_w_x_and_fmv_x_w() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        // 将整数 0x40490FDB (约等于 pi) 移到 f1
        cpu.write_reg(1, 0x40490FDB);
        let instr = RvInstr::FmvWX { frd: 1, rs1: 1 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_fp(1), 0x40490FDB);

        // 再移回整数寄存器 x2
        let instr = RvInstr::FmvXW { rd: 2, frs1: 1 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 0x40490FDB);
    }

    #[test]
    fn test_fadd_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        // f1 = 1.0, f2 = 2.0
        cpu.write_fp_f32(1, 1.0);
        cpu.write_fp_f32(2, 2.0);

        let instr = RvInstr::FaddS { frd: 3, frs1: 1, frs2: 2, rm: 0 };
        execute(&mut cpu, &mut mem, instr);

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
        execute(&mut cpu, &mut mem, instr);

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
        execute(&mut cpu, &mut mem, instr);

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
        execute(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(3);
        assert!((result - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fsqrt_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 16.0);

        let instr = RvInstr::FsqrtS { frd: 2, frs1: 1, rm: 0 };
        execute(&mut cpu, &mut mem, instr);

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

        // f4 = (2.0 * 3.0) + 1.0 = 7.0
        let instr = RvInstr::FmaddS { frd: 4, frs1: 1, frs2: 2, frs3: 3, rm: 0 };
        execute(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(4);
        assert!((result - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fcvt_w_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 42.7);

        let instr = RvInstr::FcvtWS { rd: 2, frs1: 1, rm: 0 };
        execute(&mut cpu, &mut mem, instr);

        assert_eq!(cpu.read_reg(2), 43);  // 四舍五入
    }

    #[test]
    fn test_fcvt_s_w() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_reg(1, 42);

        let instr = RvInstr::FcvtSW { frd: 1, rs1: 1, rm: 0 };
        execute(&mut cpu, &mut mem, instr);

        let result = cpu.read_fp_f32(1);
        assert!((result - 42.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fclass_s() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        // 测试正常正数
        cpu.write_fp_f32(1, 1.0);
        let instr = RvInstr::FclassS { rd: 2, frs1: 1 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 1 << 6);  // 正常正数

        // 测试正无穷
        cpu.write_fp_f32(1, f32::INFINITY);
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 1 << 7);  // +inf

        // 测试负无穷
        cpu.write_fp_f32(1, f32::NEG_INFINITY);
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(2), 1 << 0);  // -inf
    }

    #[test]
    fn test_flw_fsw() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        // 在内存中写入浮点数
        let pi_bits: u32 = std::f32::consts::PI.to_bits();
        mem.store32(0x100, pi_bits);

        // FLW: 从内存加载到 f1
        cpu.write_reg(1, 0x100);  // base address
        let instr = RvInstr::Flw { frd: 1, rs1: 1, offset: 0 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_fp(1), pi_bits);

        // FSW: 存储回内存不同地址
        cpu.write_reg(2, 0x200);
        let instr = RvInstr::Fsw { rs1: 2, frs2: 1, offset: 0 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(mem.load32(0x200), pi_bits);
    }

    #[test]
    fn test_feq_flt_fle() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 1.0);
        cpu.write_fp_f32(2, 2.0);
        cpu.write_fp_f32(3, 1.0);

        // FEQ: 1.0 == 2.0 -> 0
        let instr = RvInstr::FeqS { rd: 10, frs1: 1, frs2: 2 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 0);

        // FEQ: 1.0 == 1.0 -> 1
        let instr = RvInstr::FeqS { rd: 10, frs1: 1, frs2: 3 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 1);

        // FLT: 1.0 < 2.0 -> 1
        let instr = RvInstr::FltS { rd: 10, frs1: 1, frs2: 2 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 1);

        // FLE: 1.0 <= 1.0 -> 1
        let instr = RvInstr::FleS { rd: 10, frs1: 1, frs2: 3 };
        execute(&mut cpu, &mut mem, instr);
        assert_eq!(cpu.read_reg(10), 1);
    }

    #[test]
    fn test_fsgnj() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 3.0);
        cpu.write_fp_f32(2, -5.0);

        // FSGNJ: 取 f1 的绝对值，f2 的符号 -> -3.0
        let instr = RvInstr::FsgnjS { frd: 3, frs1: 1, frs2: 2 };
        execute(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - (-3.0)).abs() < f32::EPSILON);

        // FSGNJN: 取 f1 的绝对值，f2 符号取反 -> 3.0
        let instr = RvInstr::FsgnjnS { frd: 3, frs1: 1, frs2: 2 };
        execute(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - 3.0).abs() < f32::EPSILON);

        // FSGNJX: f1 的符号与 f2 的符号异或
        // f1=+3.0 (符号0), f2=-5.0 (符号1), 结果符号=1 -> -3.0
        let instr = RvInstr::FsgnjxS { frd: 3, frs1: 1, frs2: 2 };
        execute(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - (-3.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fmin_fmax() {
        let mut cpu = setup_fp_cpu();
        let mut mem = FlatMemory::new(0x10000, 0);

        cpu.write_fp_f32(1, 1.0);
        cpu.write_fp_f32(2, 5.0);

        // FMIN
        let instr = RvInstr::FminS { frd: 3, frs1: 1, frs2: 2 };
        execute(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - 1.0).abs() < f32::EPSILON);

        // FMAX
        let instr = RvInstr::FmaxS { frd: 3, frs1: 1, frs2: 2 };
        execute(&mut cpu, &mut mem, instr);
        let result = cpu.read_fp_f32(3);
        assert!((result - 5.0).abs() < f32::EPSILON);
    }
}
