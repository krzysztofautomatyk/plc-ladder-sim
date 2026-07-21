//! =============================================================================
//! PlcMemory — thread-safe shared process image for the PLC scan engine
//! and Modbus TCP slave. Military/medical-grade: no panic on bounds,
//! checked arithmetic at the engine layer, snapshot for SCADA/UI.
//! =============================================================================

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// Default process-image sizes (IEC-style addressable ranges).
pub const COIL_COUNT: usize = 4096;
pub const DISCRETE_INPUT_COUNT: usize = 4096;
pub const HOLDING_REGISTER_COUNT: usize = 4096;
pub const INPUT_REGISTER_COUNT: usize = 1024;
/// Internal marker bits (M) — ladder-only, never exposed on Modbus.
pub const MEMORY_BIT_COUNT: usize = 4096;
/// Internal memory registers (MR) — ladder-only, never exposed on Modbus.
pub const MEMORY_WORD_COUNT: usize = 4096;

/// Runtime status exposed to UI / Modbus diagnostic registers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlcRunState {
    #[default]
    Stop,
    Run,
    Fault,
}

/// Full memory snapshot for frontend live view and export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub coils: Vec<bool>,
    pub discrete_inputs: Vec<bool>,
    pub holding_registers: Vec<u16>,
    pub input_registers: Vec<u16>,
    #[serde(default)]
    pub memory_bits: Vec<bool>,
    #[serde(default)]
    pub memory_words: Vec<u16>,
    pub run_state: PlcRunState,
    pub scan_count: u64,
    pub last_scan_us: u64,
    pub cycle_ms: u32,
    pub program_hash: String,
    pub program_version: String,
    pub fault_code: u16,
    pub fault_message: String,
}

/// Thread-safe process image.
///
/// Coils (0x) and Holding Registers (4x) are R/W from Modbus and logic.
/// Discrete Inputs (1x) and Input Registers (3x) are typically written by
/// the scan engine (simulated field devices / diagnostics).
#[derive(Debug)]
pub struct PlcMemory {
    /// Output coils (Modbus FC 01 / 05 / 15) — also used as Q bits in LAD.
    coils: parking_lot::RwLock<Vec<bool>>,
    /// Discrete inputs (Modbus FC 02) — I bits.
    discrete_inputs: parking_lot::RwLock<Vec<bool>>,
    /// Holding registers (Modbus FC 03 / 06 / 16) — MW / timers / counters.
    holding_registers: parking_lot::RwLock<Vec<u16>>,
    /// Input registers (Modbus FC 04) — diagnostics / scan metrics.
    input_registers: parking_lot::RwLock<Vec<u16>>,
    /// Internal marker bits (M) — ladder-only working memory, never on Modbus.
    memory_bits: parking_lot::RwLock<Vec<bool>>,
    /// Internal memory registers (MR) — ladder-only working memory, never on Modbus.
    memory_words: parking_lot::RwLock<Vec<u16>>,

    run_state: parking_lot::RwLock<PlcRunState>,
    scan_count: AtomicU64,
    last_scan_us: AtomicU64,
    cycle_ms: AtomicU32,
    /// When true, Modbus writes to coils/holding are accepted.
    allow_modbus_write: AtomicBool,

    program_hash: parking_lot::RwLock<String>,
    program_version: parking_lot::RwLock<String>,
    fault_code: AtomicU32,
    fault_message: parking_lot::RwLock<String>,
}

impl Default for PlcMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl PlcMemory {
    pub fn new() -> Self {
        Self {
            coils: parking_lot::RwLock::new(vec![false; COIL_COUNT]),
            discrete_inputs: parking_lot::RwLock::new(vec![false; DISCRETE_INPUT_COUNT]),
            holding_registers: parking_lot::RwLock::new(vec![0u16; HOLDING_REGISTER_COUNT]),
            input_registers: parking_lot::RwLock::new(vec![0u16; INPUT_REGISTER_COUNT]),
            memory_bits: parking_lot::RwLock::new(vec![false; MEMORY_BIT_COUNT]),
            memory_words: parking_lot::RwLock::new(vec![0u16; MEMORY_WORD_COUNT]),
            run_state: parking_lot::RwLock::new(PlcRunState::Stop),
            scan_count: AtomicU64::new(0),
            last_scan_us: AtomicU64::new(0),
            cycle_ms: AtomicU32::new(20),
            allow_modbus_write: AtomicBool::new(false),
            program_hash: parking_lot::RwLock::new("0".repeat(64)),
            program_version: parking_lot::RwLock::new(String::from("0.0.0")),
            fault_code: AtomicU32::new(0),
            fault_message: parking_lot::RwLock::new(String::new()),
        }
    }

    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    // -------------------------------------------------------------------------
    // Coils
    // -------------------------------------------------------------------------

    pub fn get_coil(&self, addr: u16) -> Result<bool, MemoryError> {
        let coils = self.coils.read();
        coils
            .get(addr as usize)
            .copied()
            .ok_or(MemoryError::OutOfRange { area: "coil", addr })
    }

    pub fn set_coil(&self, addr: u16, value: bool) -> Result<(), MemoryError> {
        let mut coils = self.coils.write();
        let slot = coils
            .get_mut(addr as usize)
            .ok_or(MemoryError::OutOfRange { area: "coil", addr })?;
        *slot = value;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Discrete inputs
    // -------------------------------------------------------------------------

    pub fn get_discrete(&self, addr: u16) -> Result<bool, MemoryError> {
        let di = self.discrete_inputs.read();
        di.get(addr as usize)
            .copied()
            .ok_or(MemoryError::OutOfRange {
                area: "discrete_input",
                addr,
            })
    }

    pub fn set_discrete(&self, addr: u16, value: bool) -> Result<(), MemoryError> {
        let mut di = self.discrete_inputs.write();
        let slot = di.get_mut(addr as usize).ok_or(MemoryError::OutOfRange {
            area: "discrete_input",
            addr,
        })?;
        *slot = value;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Holding registers
    // -------------------------------------------------------------------------

    pub fn get_holding(&self, addr: u16) -> Result<u16, MemoryError> {
        let hr = self.holding_registers.read();
        hr.get(addr as usize)
            .copied()
            .ok_or(MemoryError::OutOfRange {
                area: "holding_register",
                addr,
            })
    }

    pub fn set_holding(&self, addr: u16, value: u16) -> Result<(), MemoryError> {
        let mut hr = self.holding_registers.write();
        let slot = hr.get_mut(addr as usize).ok_or(MemoryError::OutOfRange {
            area: "holding_register",
            addr,
        })?;
        *slot = value;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Input registers
    // -------------------------------------------------------------------------

    pub fn get_input_reg(&self, addr: u16) -> Result<u16, MemoryError> {
        let ir = self.input_registers.read();
        ir.get(addr as usize)
            .copied()
            .ok_or(MemoryError::OutOfRange {
                area: "input_register",
                addr,
            })
    }

    pub fn set_input_reg(&self, addr: u16, value: u16) -> Result<(), MemoryError> {
        let mut ir = self.input_registers.write();
        let slot = ir.get_mut(addr as usize).ok_or(MemoryError::OutOfRange {
            area: "input_register",
            addr,
        })?;
        *slot = value;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Internal memory bits (M) — ladder-only, never on Modbus
    // -------------------------------------------------------------------------

    pub fn get_memory_bit(&self, addr: u16) -> Result<bool, MemoryError> {
        let m = self.memory_bits.read();
        m.get(addr as usize)
            .copied()
            .ok_or(MemoryError::OutOfRange {
                area: "memory_bit",
                addr,
            })
    }

    pub fn set_memory_bit(&self, addr: u16, value: bool) -> Result<(), MemoryError> {
        let mut m = self.memory_bits.write();
        let slot = m.get_mut(addr as usize).ok_or(MemoryError::OutOfRange {
            area: "memory_bit",
            addr,
        })?;
        *slot = value;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Internal memory registers (MR) — ladder-only, never on Modbus
    // -------------------------------------------------------------------------

    pub fn get_memory_word(&self, addr: u16) -> Result<u16, MemoryError> {
        let m = self.memory_words.read();
        m.get(addr as usize)
            .copied()
            .ok_or(MemoryError::OutOfRange {
                area: "memory_word",
                addr,
            })
    }

    pub fn set_memory_word(&self, addr: u16, value: u16) -> Result<(), MemoryError> {
        let mut m = self.memory_words.write();
        let slot = m.get_mut(addr as usize).ok_or(MemoryError::OutOfRange {
            area: "memory_word",
            addr,
        })?;
        *slot = value;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Runtime metadata
    // -------------------------------------------------------------------------

    pub fn set_run_state(&self, state: PlcRunState) {
        *self.run_state.write() = state;
        // Diagnostic IR0 = run state code
        let code = match state {
            PlcRunState::Stop => 0u16,
            PlcRunState::Run => 1,
            PlcRunState::Fault => 2,
        };
        let _ = self.set_input_reg(0, code);
    }

    pub fn run_state(&self) -> PlcRunState {
        *self.run_state.read()
    }

    pub fn set_cycle_ms(&self, ms: u32) {
        let clamped = ms.clamp(5, 100);
        self.cycle_ms.store(clamped, Ordering::Relaxed);
        let _ = self.set_input_reg(1, clamped as u16);
    }

    pub fn cycle_ms(&self) -> u32 {
        self.cycle_ms.load(Ordering::Relaxed)
    }

    pub fn increment_scan(&self, scan_us: u64) {
        self.scan_count.fetch_add(1, Ordering::Relaxed);
        self.last_scan_us.store(scan_us, Ordering::Relaxed);
        let count = self.scan_count.load(Ordering::Relaxed);
        let _ = self.set_input_reg(2, (count & 0xFFFF) as u16);
        let _ = self.set_input_reg(3, ((count >> 16) & 0xFFFF) as u16);
        let _ = self.set_input_reg(4, scan_us.min(u16::MAX as u64) as u16);
    }

    pub fn scan_count(&self) -> u64 {
        self.scan_count.load(Ordering::Relaxed)
    }

    pub fn last_scan_us(&self) -> u64 {
        self.last_scan_us.load(Ordering::Relaxed)
    }

    pub fn set_program_meta(&self, hash: String, version: String) {
        *self.program_hash.write() = hash;
        *self.program_version.write() = version;
    }

    pub fn program_hash(&self) -> String {
        self.program_hash.read().clone()
    }

    pub fn program_version(&self) -> String {
        self.program_version.read().clone()
    }

    pub fn raise_fault(&self, code: u16, message: impl Into<String>) {
        self.fault_code.store(code as u32, Ordering::Relaxed);
        *self.fault_message.write() = message.into();
        self.set_run_state(PlcRunState::Fault);
        let _ = self.set_input_reg(5, code);
    }

    pub fn clear_fault(&self) {
        self.fault_code.store(0, Ordering::Relaxed);
        *self.fault_message.write() = String::new();
        let _ = self.set_input_reg(5, 0);
    }

    pub fn fault_code(&self) -> u16 {
        self.fault_code.load(Ordering::Relaxed) as u16
    }

    pub fn allow_modbus_write(&self) -> bool {
        self.allow_modbus_write.load(Ordering::Relaxed)
    }

    pub fn set_allow_modbus_write(&self, allow: bool) {
        self.allow_modbus_write.store(allow, Ordering::Relaxed);
    }

    /// Reset process image (does not clear program metadata).
    pub fn reset_process_image(&self) {
        {
            let mut c = self.coils.write();
            c.fill(false);
        }
        {
            let mut d = self.discrete_inputs.write();
            d.fill(false);
        }
        {
            let mut h = self.holding_registers.write();
            h.fill(0);
        }
        {
            let mut i = self.input_registers.write();
            i.fill(0);
        }
        {
            let mut m = self.memory_bits.write();
            m.fill(false);
        }
        {
            let mut m = self.memory_words.write();
            m.fill(0);
        }
        self.scan_count.store(0, Ordering::Relaxed);
        self.last_scan_us.store(0, Ordering::Relaxed);
        self.clear_fault();
        self.set_run_state(PlcRunState::Stop);
        let cycle = self.cycle_ms();
        self.set_cycle_ms(cycle);
    }

    pub fn snapshot(&self) -> MemorySnapshot {
        // Acquire all four process-image locks together in a fixed order so the
        // snapshot is a *coherent* image: coils and registers are cloned while
        // every area is held, so a concurrent single-area write can never tear
        // the snapshot across areas. Writers only ever take one area lock at a
        // time, so this ordering cannot deadlock.
        let coils = self.coils.read();
        let discrete_inputs = self.discrete_inputs.read();
        let holding_registers = self.holding_registers.read();
        let input_registers = self.input_registers.read();
        let memory_bits = self.memory_bits.read();
        let memory_words = self.memory_words.read();
        MemorySnapshot {
            coils: coils.clone(),
            discrete_inputs: discrete_inputs.clone(),
            holding_registers: holding_registers.clone(),
            input_registers: input_registers.clone(),
            memory_bits: memory_bits.clone(),
            memory_words: memory_words.clone(),
            run_state: self.run_state(),
            scan_count: self.scan_count(),
            last_scan_us: self.last_scan_us(),
            cycle_ms: self.cycle_ms(),
            program_hash: self.program_hash(),
            program_version: self.program_version(),
            fault_code: self.fault_code(),
            fault_message: self.fault_message.read().clone(),
        }
    }

    /// Compact snapshot for high-frequency UI updates (first N of each area).
    pub fn snapshot_compact(&self, coil_n: usize, reg_n: usize) -> MemorySnapshot {
        let full = self.snapshot();
        MemorySnapshot {
            coils: full.coils.into_iter().take(coil_n).collect(),
            discrete_inputs: full.discrete_inputs.into_iter().take(coil_n).collect(),
            holding_registers: full.holding_registers.into_iter().take(reg_n).collect(),
            input_registers: full
                .input_registers
                .into_iter()
                .take(reg_n.min(16))
                .collect(),
            memory_bits: full.memory_bits.into_iter().take(coil_n).collect(),
            memory_words: full.memory_words.into_iter().take(reg_n).collect(),
            ..full
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("address out of range in {area}: {addr}")]
    OutOfRange { area: &'static str, addr: u16 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coil_roundtrip() {
        let m = PlcMemory::new();
        m.set_coil(10, true).unwrap();
        assert!(m.get_coil(10).unwrap());
        assert!(!m.get_coil(11).unwrap());
    }

    #[test]
    fn holding_bounds() {
        let m = PlcMemory::new();
        assert!(m.get_holding(HOLDING_REGISTER_COUNT as u16).is_err());
    }

    #[test]
    fn cycle_clamped() {
        let m = PlcMemory::new();
        m.set_cycle_ms(1);
        assert_eq!(m.cycle_ms(), 5);
        m.set_cycle_ms(500);
        assert_eq!(m.cycle_ms(), 100);
    }

    #[test]
    fn snapshot_is_coherent_across_areas() {
        let m = PlcMemory::new();
        m.set_coil(1, true).unwrap();
        m.set_discrete(2, true).unwrap();
        m.set_holding(3, 0xBEEF).unwrap();
        m.set_input_reg(4, 0x1234).unwrap();

        let snap = m.snapshot();
        assert_eq!(snap.coils.len(), COIL_COUNT);
        assert_eq!(snap.discrete_inputs.len(), DISCRETE_INPUT_COUNT);
        assert_eq!(snap.holding_registers.len(), HOLDING_REGISTER_COUNT);
        assert_eq!(snap.input_registers.len(), INPUT_REGISTER_COUNT);
        assert!(snap.coils[1]);
        assert!(snap.discrete_inputs[2]);
        assert_eq!(snap.holding_registers[3], 0xBEEF);
        assert_eq!(snap.input_registers[4], 0x1234);
    }

    #[test]
    fn concurrent_writes_never_deadlock_snapshot() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let m = PlcMemory::new().into_arc();
        let stop = Arc::new(AtomicBool::new(false));

        let mw = Arc::clone(&m);
        let sw = Arc::clone(&stop);
        let writer = std::thread::spawn(move || {
            let mut i: u16 = 0;
            while !sw.load(Ordering::Relaxed) {
                let _ = mw.set_coil(i % 64, i % 2 == 0);
                let _ = mw.set_holding(i % 64, i);
                i = i.wrapping_add(1);
            }
        });

        // Many snapshots concurrent with writes must always complete.
        for _ in 0..2000 {
            let s = m.snapshot();
            assert_eq!(s.coils.len(), COIL_COUNT);
        }
        stop.store(true, Ordering::Relaxed);
        writer.join().unwrap();
    }
}
