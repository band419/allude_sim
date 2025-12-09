//! CPU architectural state components: register file(s) and CSR bank.

use std::collections::HashMap;
use super::trap::PrivilegeMode;

/// Generic register file with configurable count, element type, and zero-hardwire behavior.
///
/// - `N`: number of registers
/// - `T`: element type (u32, u64, etc.)
/// - `ZERO_HARDWIRE`: if true, register 0 always reads as zero and writes are ignored
#[derive(Clone)]
pub struct GenericRegFile<const N: usize, T: Copy + Default, const ZERO_HARDWIRE: bool> {
    regs: [T; N],
}

impl<const N: usize, T: Copy + Default, const ZERO_HARDWIRE: bool> GenericRegFile<N, T, ZERO_HARDWIRE>
where
    [T; N]: Default,
{
    pub fn new() -> Self {
        Self { regs: [T::default(); N] }
    }

    #[inline]
    pub fn read(&self, reg: u8) -> T {
        if ZERO_HARDWIRE && reg == 0 {
            T::default()
        } else {
            self.regs[reg as usize]
        }
    }

    #[inline]
    pub fn write(&mut self, reg: u8, value: T) {
        if ZERO_HARDWIRE && reg == 0 {
            return;
        }
        self.regs[reg as usize] = value;
    }

    pub fn snapshot(&self) -> &[T; N] {
        &self.regs
    }
}

impl<const N: usize, T: Copy + Default, const ZERO_HARDWIRE: bool> Default for GenericRegFile<N, T, ZERO_HARDWIRE>
where
    [T; N]: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Integer register file x0..x31. x0 is hard-wired to zero.
pub type RegFile = GenericRegFile<32, u32, true>;

/// Floating-point register file f0..f31. No zero-hardwire.
pub type FpRegFile = GenericRegFile<32, u32, false>;

/// Vector register file v0..v31. Each element is 128-bit (VLEN=128 default).
/// Stored as [u8; 16] per register for flexibility.
pub type VecRegFile = GenericRegFile<32, [u8; 16], false>;

/// Table entry for CSR declaration: name, address, reset value.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct CsrEntry {
    pub name: &'static str,
    pub addr: u16,
    pub reset: u32,
}

/// Simple CSR bank: a hash table indexed by address.
#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct CsrBank {
    table: HashMap<u16, u32>,
}

impl CsrBank {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    /// Register a batch of CSRs declared as a table.
    #[allow(dead_code)]
    pub fn register(&mut self, entries: &[CsrEntry]) {
        for e in entries {
            self.table.insert(e.addr, e.reset);
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn read(&self, addr: u16) -> u32 {
        *self.table.get(&addr).unwrap_or(&0)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn write(&mut self, addr: u16, value: u32) {
        self.table.insert(addr, value);
    }

    #[inline]
    #[allow(dead_code)]
    pub fn snapshot(&self) -> &HashMap<u16, u32> {
        &self.table
    }
}

/// Aggregated architectural state: integer RF, optional FP/Vec RF, and CSR bank.
#[derive(Clone)]
pub struct Status {
    pub int: RegFile,
    #[allow(dead_code)]
    pub fp: Option<FpRegFile>,
    #[allow(dead_code)]
    pub vec: Option<VecRegFile>,
    #[allow(dead_code)]
    pub csr: CsrBank,
    /// Current privilege mode
    pub privilege: PrivilegeMode,
}

impl Default for Status {
    fn default() -> Self {
        Self::new()
    }
}

impl Status {
    pub fn new() -> Self {
        Self {
            int: RegFile::new(),
            fp: None,
            vec: None,
            csr: CsrBank::new(),
            privilege: PrivilegeMode::Machine, // 启动时为 M-mode
        }
    }

    /// Enable floating-point state (F extension) on demand.
    #[allow(dead_code)]
    pub fn enable_fp(&mut self) {
        if self.fp.is_none() {
            self.fp = Some(FpRegFile::new());
        }
    }

    /// Enable vector state (V extension) on demand.
    #[allow(dead_code)]
    pub fn enable_vec(&mut self) {
        if self.vec.is_none() {
            self.vec = Some(VecRegFile::new());
        }
    }

    // Integer register file access
    #[inline]
    pub fn int_read(&self, reg: u8) -> u32 {
        self.int.read(reg)
    }

    #[inline]
    pub fn int_write(&mut self, reg: u8, value: u32) {
        self.int.write(reg, value)
    }

    #[inline]
    pub fn int_snapshot(&self) -> &[u32; 32] {
        self.int.snapshot()
    }

    // Floating-point register file access (returns Option for optional F extension)
    #[inline]
    #[allow(dead_code)]
    pub fn fp_read(&self, reg: u8) -> Option<u32> {
        self.fp.as_ref().map(|f| f.read(reg))
    }

    #[inline]
    #[allow(dead_code)]
    pub fn fp_write(&mut self, reg: u8, value: u32) -> bool {
        if let Some(f) = self.fp.as_mut() {
            f.write(reg, value);
            true
        } else {
            false
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn fp_snapshot(&self) -> Option<&[u32; 32]> {
        self.fp.as_ref().map(|f| f.snapshot())
    }

    // Vector register file access (returns Option for optional V extension)
    #[inline]
    #[allow(dead_code)]
    pub fn vec_read(&self, reg: u8) -> Option<[u8; 16]> {
        self.vec.as_ref().map(|v| v.read(reg))
    }

    #[inline]
    #[allow(dead_code)]
    pub fn vec_write(&mut self, reg: u8, value: [u8; 16]) -> bool {
        if let Some(v) = self.vec.as_mut() {
            v.write(reg, value);
            true
        } else {
            false
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn vec_snapshot(&self) -> Option<&[[u8; 16]; 32]> {
        self.vec.as_ref().map(|v| v.snapshot())
    }

    // CSR access
    #[inline]
    #[allow(dead_code)]
    pub fn csr_read(&self, addr: u16) -> u32 {
        self.csr.read(addr)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn csr_write(&mut self, addr: u16, value: u32) {
        self.csr.write(addr, value)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn csr_snapshot(&self) -> &HashMap<u16, u32> {
        self.csr.snapshot()
    }

    /// Snapshot all architectural state at once.
    #[allow(dead_code)]
    pub fn snapshot(&self) -> StatusSnapshot {
        StatusSnapshot {
            int: self.int.snapshot().clone(),
            fp: self.fp.as_ref().map(|f| f.snapshot().clone()),
            vec: self.vec.as_ref().map(|v| v.snapshot().clone()),
            csr: self.csr.table.clone(),
        }
    }
}

/// Snapshot of all architectural state.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct StatusSnapshot {
    pub int: [u32; 32],
    pub fp: Option<[u32; 32]>,
    pub vec: Option<[[u8; 16]; 32]>,
    pub csr: HashMap<u16, u32>,
}
