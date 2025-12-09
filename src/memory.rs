//! 内存抽象层
//!
//! 本模块定义了内存访问的统一接口 `Memory` trait，
//! 以及用于功能验证的简单线性内存实现 `FlatMemory`。

/// 访存粒度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSize {
    Byte,
    Half,
    Word,
}

impl AccessSize {
    pub fn bytes(self) -> usize {
        match self {
            AccessSize::Byte => 1,
            AccessSize::Half => 2,
            AccessSize::Word => 4,
        }
    }
}

/// 内存访问错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemError {
    /// 地址未按访问粒度对齐
    Unaligned { addr: u32, access: AccessSize },
    /// 地址越界（未映射到当前内存区域）
    OutOfRange { addr: u32, access: AccessSize, base: u32, size: usize },
}

impl std::fmt::Display for MemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemError::Unaligned { addr, access } => {
                write!(f, "Unaligned {:?} access at 0x{:08x}", access, addr)
            }
            MemError::OutOfRange { addr, access, base, size } => {
                write!(
                    f,
                    "Out-of-range {:?} access at 0x{:08x} (region=0x{:08x}..0x{:08x})",
                    access,
                    addr,
                    base,
                    base.wrapping_add(*size as u32)
                )
            }
        }
    }
}

impl std::error::Error for MemError {}

pub type MemResult<T> = Result<T, MemError>;

/// 内存访问的统一接口
///
/// 为方便后续接入多种内存模型（平坦 DRAM、cache 分层、共享内存等），
/// 定义统一内存接口。
pub trait Memory {
    /// 从指定地址读取 8 位数据
    fn load8(&self, addr: u32) -> MemResult<u8>;

    /// 从指定地址读取 16 位数据（小端序）
    fn load16(&self, addr: u32) -> MemResult<u16>;

    /// 从指定地址读取 32 位数据（小端序）
    fn load32(&self, addr: u32) -> MemResult<u32>;

    /// 向指定地址写入 8 位数据
    fn store8(&mut self, addr: u32, value: u8) -> MemResult<()>;

    /// 向指定地址写入 16 位数据（小端序）
    fn store16(&mut self, addr: u32, value: u16) -> MemResult<()>;

    /// 向指定地址写入 32 位数据（小端序）
    fn store32(&mut self, addr: u32, value: u32) -> MemResult<()>;
}

/// 简单线性内存实现
///
/// 使用 `Vec<u8>` 存储整个地址空间或一段工作集。
/// 支持可选的基地址偏移，用于模拟内存映射。
///
/// 特点：
/// - 实现简单，便于先验证 ISA 逻辑
/// - 后续可以替换/包装为更复杂的内存体系结构，而不影响 CPU 与 ISA 层代码
pub struct FlatMemory {
    /// 内存数据存储
    data: Vec<u8>,
    /// 内存映射起始地址
    base_addr: u32,
}

impl FlatMemory {
    /// 创建一个指定大小的内存区域
    ///
    /// # 参数
    ///
    /// * `size` - 内存大小（字节数）
    /// * `base_addr` - 内存映射的起始地址
    ///
    /// # 示例
    ///
    /// ```
    /// use allude_sim::memory::FlatMemory;
    ///
    /// // 创建 64KB 的内存，起始地址为 0
    /// let mem = FlatMemory::new(64 * 1024, 0);
    /// ```
    pub fn new(size: usize, base_addr: u32) -> Self {
        FlatMemory {
            data: vec![0; size],
            base_addr,
        }
    }

    /// 获取内存的基地址
    pub fn base_addr(&self) -> u32 {
        self.base_addr
    }

    /// 获取内存的大小
    pub fn size(&self) -> usize {
        self.data.len()
    }

    fn ensure_aligned(addr: u32, access: AccessSize) -> MemResult<()> {
        match access {
            AccessSize::Byte => Ok(()),
            AccessSize::Half if addr.is_multiple_of(2) => Ok(()),
            AccessSize::Word if addr.is_multiple_of(4) => Ok(()),
            _ => Err(MemError::Unaligned { addr, access }),
        }
    }

    fn bounds_check(&self, addr: u32, len: usize, access: AccessSize) -> MemResult<usize> {
        let relative = addr
            .checked_sub(self.base_addr)
            .ok_or(MemError::OutOfRange {
                addr,
                access,
                base: self.base_addr,
                size: self.data.len(),
            })? as usize;

        let end = relative
            .checked_add(len)
            .ok_or(MemError::OutOfRange {
                addr,
                access,
                base: self.base_addr,
                size: self.data.len(),
            })?;

        if end > self.data.len() {
            return Err(MemError::OutOfRange {
                addr,
                access,
                base: self.base_addr,
                size: self.data.len(),
            });
        }

        Ok(relative)
    }

    /// 批量写入数据到内存
    ///
    /// # 参数
    ///
    /// * `addr` - 起始地址
    /// * `data` - 要写入的数据
    ///
    /// # Panics
    ///
    /// 如果写入范围超出内存，将 panic
    pub fn write_bytes(&mut self, addr: u32, data: &[u8]) -> MemResult<()> {
        if data.is_empty() {
            return Ok(());
        }
        let start = self.bounds_check(addr, data.len(), AccessSize::Byte)?;
        let end = start + data.len();
        self.data[start..end].copy_from_slice(data);
        Ok(())
    }

    /// 批量读取数据
    ///
    /// # 参数
    ///
    /// * `addr` - 起始地址
    /// * `len` - 读取长度
    ///
    /// # 返回
    ///
    /// 读取的数据切片的副本
    pub fn read_bytes(&self, addr: u32, len: usize) -> MemResult<Vec<u8>> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let start = self.bounds_check(addr, len, AccessSize::Byte)?;
        let end = start + len;
        Ok(self.data[start..end].to_vec())
    }

    /// 将指定范围填充为固定字节
    pub fn fill(&mut self, addr: u32, len: usize, value: u8) -> MemResult<()> {
        if len == 0 {
            return Ok(());
        }
        let start = self.bounds_check(addr, len, AccessSize::Byte)?;
        let end = start + len;
        self.data[start..end].fill(value);
        Ok(())
    }
}

impl Memory for FlatMemory {
    fn load8(&self, addr: u32) -> MemResult<u8> {
        let idx = self.bounds_check(addr, 1, AccessSize::Byte)?;
        Ok(self.data[idx])
    }

    fn load16(&self, addr: u32) -> MemResult<u16> {
        Self::ensure_aligned(addr, AccessSize::Half)?;
        let idx = self.bounds_check(addr, 2, AccessSize::Half)?;
        Ok(u16::from_le_bytes([self.data[idx], self.data[idx + 1]]))
    }

    fn load32(&self, addr: u32) -> MemResult<u32> {
        Self::ensure_aligned(addr, AccessSize::Word)?;
        let idx = self.bounds_check(addr, 4, AccessSize::Word)?;
        Ok(u32::from_le_bytes([
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ]))
    }

    fn store8(&mut self, addr: u32, value: u8) -> MemResult<()> {
        let idx = self.bounds_check(addr, 1, AccessSize::Byte)?;
        self.data[idx] = value;
        Ok(())
    }

    fn store16(&mut self, addr: u32, value: u16) -> MemResult<()> {
        Self::ensure_aligned(addr, AccessSize::Half)?;
        let idx = self.bounds_check(addr, 2, AccessSize::Half)?;
        let bytes = value.to_le_bytes();
        self.data[idx] = bytes[0];
        self.data[idx + 1] = bytes[1];
        Ok(())
    }

    fn store32(&mut self, addr: u32, value: u32) -> MemResult<()> {
        Self::ensure_aligned(addr, AccessSize::Word)?;
        let idx = self.bounds_check(addr, 4, AccessSize::Word)?;
        let bytes = value.to_le_bytes();
        self.data[idx] = bytes[0];
        self.data[idx + 1] = bytes[1];
        self.data[idx + 2] = bytes[2];
        self.data[idx + 3] = bytes[3];
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_memory_basic() {
        let mut mem = FlatMemory::new(1024, 0);

        // 测试 8 位读写
        mem.store8(0, 0x12).unwrap();
        assert_eq!(mem.load8(0).unwrap(), 0x12);

        // 测试 16 位读写（小端序）
        mem.store16(2, 0x3456).unwrap();
        assert_eq!(mem.load16(2).unwrap(), 0x3456);
        assert_eq!(mem.load8(2).unwrap(), 0x56); // 低字节
        assert_eq!(mem.load8(3).unwrap(), 0x34); // 高字节

        // 测试 32 位读写（小端序）
        mem.store32(4, 0x78ABCDEF).unwrap();
        assert_eq!(mem.load32(4).unwrap(), 0x78ABCDEF);
        assert_eq!(mem.load8(4).unwrap(), 0xEF); // 最低字节
        assert_eq!(mem.load8(5).unwrap(), 0xCD);
        assert_eq!(mem.load8(6).unwrap(), 0xAB);
        assert_eq!(mem.load8(7).unwrap(), 0x78); // 最高字节
    }

    #[test]
    fn test_flat_memory_with_base_addr() {
        let mut mem = FlatMemory::new(1024, 0x1000);

        mem.store32(0x1000, 0xDEADBEEF).unwrap();
        assert_eq!(mem.load32(0x1000).unwrap(), 0xDEADBEEF);

        mem.store8(0x1004, 0x42).unwrap();
        assert_eq!(mem.load8(0x1004).unwrap(), 0x42);
    }

    #[test]
    fn test_write_bytes() {
        let mut mem = FlatMemory::new(1024, 0);
        let data = [0x01, 0x02, 0x03, 0x04];
        mem.write_bytes(0, &data).unwrap();

        assert_eq!(mem.load8(0).unwrap(), 0x01);
        assert_eq!(mem.load8(1).unwrap(), 0x02);
        assert_eq!(mem.load8(2).unwrap(), 0x03);
        assert_eq!(mem.load8(3).unwrap(), 0x04);
        assert_eq!(mem.load32(0).unwrap(), 0x04030201); // 小端序
    }

    #[test]
    fn test_unaligned_load16() {
        let mem = FlatMemory::new(1024, 0);
        let err = mem.load16(1).unwrap_err();
        assert!(matches!(err, MemError::Unaligned { .. }));
    }

    #[test]
    fn test_unaligned_load32() {
        let mem = FlatMemory::new(1024, 0);
        let err = mem.load32(1).unwrap_err();
        assert!(matches!(err, MemError::Unaligned { .. }));
    }

    #[test]
    fn test_out_of_bounds() {
        let mem = FlatMemory::new(1024, 0);
        let err = mem.load8(2000).unwrap_err();
        assert!(matches!(err, MemError::OutOfRange { .. }));
    }
}
