//! 内存抽象层
//!
//! 本模块定义了内存访问的统一接口 `Memory` trait，
//! 以及用于功能验证的简单线性内存实现 `FlatMemory`。

/// 内存访问的统一接口
///
/// 为方便后续接入多种内存模型（平坦 DRAM、cache 分层、共享内存等），
/// 定义统一内存接口。
///
/// 设计要点：
/// - 地址使用 `u32`，与 RV32I 匹配
/// - 按最常见的 8/16/32-bit 访存粒度建接口
/// - 不假设底层物理实现，可以是简单数组、分段、或带统计的模型
pub trait Memory {
    /// 从指定地址读取 8 位数据
    fn load8(&self, addr: u32) -> u8;

    /// 从指定地址读取 16 位数据（小端序）
    fn load16(&self, addr: u32) -> u16;

    /// 从指定地址读取 32 位数据（小端序）
    fn load32(&self, addr: u32) -> u32;

    /// 向指定地址写入 8 位数据
    fn store8(&mut self, addr: u32, value: u8);

    /// 向指定地址写入 16 位数据（小端序）
    fn store16(&mut self, addr: u32, value: u16);

    /// 向指定地址写入 32 位数据（小端序）
    fn store32(&mut self, addr: u32, value: u32);
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

    /// 将地址转换为内部索引
    ///
    /// # Panics
    ///
    /// 如果地址超出内存范围，将 panic
    fn addr_to_index(&self, addr: u32) -> usize {
        let offset = addr.wrapping_sub(self.base_addr) as usize;
        if offset >= self.data.len() {
            panic!(
                "Memory access out of bounds: addr=0x{:08x}, base=0x{:08x}, size=0x{:x}",
                addr,
                self.base_addr,
                self.data.len()
            );
        }
        offset
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
    pub fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        let start = self.addr_to_index(addr);
        let end = start + data.len();
        if end > self.data.len() {
            panic!(
                "Memory write out of bounds: addr=0x{:08x}, len={}, size=0x{:x}",
                addr,
                data.len(),
                self.data.len()
            );
        }
        self.data[start..end].copy_from_slice(data);
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
    pub fn read_bytes(&self, addr: u32, len: usize) -> Vec<u8> {
        let start = self.addr_to_index(addr);
        let end = start + len;
        if end > self.data.len() {
            panic!(
                "Memory read out of bounds: addr=0x{:08x}, len={}, size=0x{:x}",
                addr,
                len,
                self.data.len()
            );
        }
        self.data[start..end].to_vec()
    }
}

impl Memory for FlatMemory {
    fn load8(&self, addr: u32) -> u8 {
        let idx = self.addr_to_index(addr);
        self.data[idx]
    }

    fn load16(&self, addr: u32) -> u16 {
        // 检查对齐
        if !addr.is_multiple_of(2) {
            panic!("Unaligned halfword load at address 0x{:08x}", addr);
        }
        let idx = self.addr_to_index(addr);
        // 小端序：低地址存放低字节
        u16::from_le_bytes([self.data[idx], self.data[idx + 1]])
    }

    fn load32(&self, addr: u32) -> u32 {
        // 检查对齐
        if !addr.is_multiple_of(4) {
            panic!("Unaligned word load at address 0x{:08x}", addr);
        }
        let idx = self.addr_to_index(addr);
        // 小端序
        u32::from_le_bytes([
            self.data[idx],
            self.data[idx + 1],
            self.data[idx + 2],
            self.data[idx + 3],
        ])
    }

    fn store8(&mut self, addr: u32, value: u8) {
        let idx = self.addr_to_index(addr);
        self.data[idx] = value;
    }

    fn store16(&mut self, addr: u32, value: u16) {
        // 检查对齐
        if !addr.is_multiple_of(2) {
            panic!("Unaligned halfword store at address 0x{:08x}", addr);
        }
        let idx = self.addr_to_index(addr);
        let bytes = value.to_le_bytes();
        self.data[idx] = bytes[0];
        self.data[idx + 1] = bytes[1];
    }

    fn store32(&mut self, addr: u32, value: u32) {
        // 检查对齐
        if !addr.is_multiple_of(4) {
            panic!("Unaligned word store at address 0x{:08x}", addr);
        }
        let idx = self.addr_to_index(addr);
        let bytes = value.to_le_bytes();
        self.data[idx] = bytes[0];
        self.data[idx + 1] = bytes[1];
        self.data[idx + 2] = bytes[2];
        self.data[idx + 3] = bytes[3];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_memory_basic() {
        let mut mem = FlatMemory::new(1024, 0);

        // 测试 8 位读写
        mem.store8(0, 0x12);
        assert_eq!(mem.load8(0), 0x12);

        // 测试 16 位读写（小端序）
        mem.store16(2, 0x3456);
        assert_eq!(mem.load16(2), 0x3456);
        assert_eq!(mem.load8(2), 0x56); // 低字节
        assert_eq!(mem.load8(3), 0x34); // 高字节

        // 测试 32 位读写（小端序）
        mem.store32(4, 0x78ABCDEF);
        assert_eq!(mem.load32(4), 0x78ABCDEF);
        assert_eq!(mem.load8(4), 0xEF); // 最低字节
        assert_eq!(mem.load8(5), 0xCD);
        assert_eq!(mem.load8(6), 0xAB);
        assert_eq!(mem.load8(7), 0x78); // 最高字节
    }

    #[test]
    fn test_flat_memory_with_base_addr() {
        let mut mem = FlatMemory::new(1024, 0x1000);

        mem.store32(0x1000, 0xDEADBEEF);
        assert_eq!(mem.load32(0x1000), 0xDEADBEEF);

        mem.store8(0x1004, 0x42);
        assert_eq!(mem.load8(0x1004), 0x42);
    }

    #[test]
    fn test_write_bytes() {
        let mut mem = FlatMemory::new(1024, 0);
        let data = [0x01, 0x02, 0x03, 0x04];
        mem.write_bytes(0, &data);

        assert_eq!(mem.load8(0), 0x01);
        assert_eq!(mem.load8(1), 0x02);
        assert_eq!(mem.load8(2), 0x03);
        assert_eq!(mem.load8(3), 0x04);
        assert_eq!(mem.load32(0), 0x04030201); // 小端序
    }

    #[test]
    #[should_panic(expected = "Unaligned")]
    fn test_unaligned_load16() {
        let mem = FlatMemory::new(1024, 0);
        mem.load16(1); // 未对齐的地址
    }

    #[test]
    #[should_panic(expected = "Unaligned")]
    fn test_unaligned_load32() {
        let mem = FlatMemory::new(1024, 0);
        mem.load32(1); // 未对齐的地址
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn test_out_of_bounds() {
        let mem = FlatMemory::new(1024, 0);
        mem.load8(2000); // 超出范围
    }
}
