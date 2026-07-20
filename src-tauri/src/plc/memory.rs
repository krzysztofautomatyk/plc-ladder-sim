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
pub const HOLDING_REGISTER_COUNT: usize = 1024;
pub const INPUT_REGISTER_COUNT: usize = 1024;

/// Runtime status exposed to UI / Modbus diagnostic registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlcRunState {
    Stop,
    Run,
    Fault,
}

impl Default for PlcRunState {
    fn default() -> Self {
        Self::Stop
    }
}

/// Full memory snapshot for frontend live view and export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub coils: Vec<bool>,
    pub discrete_inputs: Vec<bool>,
    pub holding_registers: Vec<u16>,
    pub input_registers: Vec<u16>,
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
            run_state: parking_lot::RwLock::new(PlcRunState::Stop),
            scan_count: AtomicU64::new(0),
            last_scan_us: AtomicU64::new(0),
            cycle_ms: AtomicU32::new(20),
            allow_modbus_write: AtomicBool::new(true),
            program_hash: parking_lot::RwLock::new(String::from("0".repeat(64))),
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
            .ok_or(MemoryError::OutOfRange {
                area: "coil",
                addr,
            })
    }

    pub fn set_coil(&self, addr: u16, value: bool) -> Result<(), MemoryError> {
        let mut coils = self.coils.write();
        let slot = coils.get_mut(addr as usize).ok_or(MemoryError::OutOfRange {
            area: "coil",
            addr,
        })?;
        *slot = value;
        Ok(())
    }

    pub fn read_coils(&self, start: u16, qty: u16) -> Result<Vec<bool>, MemoryError> {
        self.read_bool_range(&self.coils, start, qty, "coil")
    }

    pub fn write_coils(&self, start: u16, values: &[bool]) -> Result<(), MemoryError> {
        self.write_bool_range(&self.coils, start, values, "coil")
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

    pub fn read_discretes(&self, start: u16, qty: u16) -> Result<Vec<bool>, MemoryError> {
        self.read_bool_range(&self.discrete_inputs, start, qty, "discrete_input")
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

    pub fn read_holdings(&self, start: u16, qty: u16) -> Result<Vec<u16>, MemoryError> {
        self.read_u16_range(&self.holding_registers, start, qty, "holding_register")
    }

    pub fn write_holdings(&self, start: u16, values: &[u16]) -> Result<(), MemoryError> {
        self.write_u16_range(&self.holding_registers, start, values, "holding_register")
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

    pub fn read_input_regs(&self, start: u16, qty: u16) -> Result<Vec<u16>, MemoryError> {
        self.read_u16_range(&self.input_registers, start, qty, "input_register")
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
        self.scan_count.store(0, Ordering::Relaxed);
        self.last_scan_us.store(0, Ordering::Relaxed);
        self.clear_fault();
        self.set_run_state(PlcRunState::Stop);
        let cycle = self.cycle_ms();
        self.set_cycle_ms(cycle);
    }

    pub fn snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            coils: self.coils.read().clone(),
            discrete_inputs: self.discrete_inputs.read().clone(),
            holding_registers: self.holding_registers.read().clone(),
            input_registers: self.input_registers.read().clone(),
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
            input_registers: full.input_registers.into_iter().take(reg_n.min(16)).collect(),
            ..full
        }
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    fn read_bool_range(
        &self,
        lock: &parking_lot::RwLock<Vec<bool>>,
        start: u16,
        qty: u16,
        area: &'static str,
    ) -> Result<Vec<bool>, MemoryError> {
        if qty == 0 {
            return Ok(Vec::new());
        }
        let data = lock.read();
        let start_u = start as usize;
        let end = start_u
            .checked_add(qty as usize)
            .ok_or(MemoryError::Overflow)?;
        if end > data.len() {
            return Err(MemoryError::OutOfRange { area, addr: start });
        }
        Ok(data[start_u..end].to_vec())
    }

    fn write_bool_range(
        &self,
        lock: &parking_lot::RwLock<Vec<bool>>,
        start: u16,
        values: &[bool],
        area: &'static str,
    ) -> Result<(), MemoryError> {
        if values.is_empty() {
            return Ok(());
        }
        let mut data = lock.write();
        let start_u = start as usize;
        let end = start_u
            .checked_add(values.len())
            .ok_or(MemoryError::Overflow)?;
        if end > data.len() {
            return Err(MemoryError::OutOfRange { area, addr: start });
        }
        data[start_u..end].copy_from_slice(values);
        Ok(())
    }

    fn read_u16_range(
        &self,
        lock: &parking_lot::RwLock<Vec<u16>>,
        start: u16,
        qty: u16,
        area: &'static str,
    ) -> Result<Vec<u16>, MemoryError> {
        if qty == 0 {
            return Ok(Vec::new());
        }
        let data = lock.read();
        let start_u = start as usize;
        let end = start_u
            .checked_add(qty as usize)
            .ok_or(MemoryError::Overflow)?;
        if end > data.len() {
            return Err(MemoryError::OutOfRange { area, addr: start });
        }
        Ok(data[start_u..end].to_vec())
    }

    fn write_u16_range(
        &self,
        lock: &parking_lot::RwLock<Vec<u16>>,
        start: u16,
        values: &[u16],
        area: &'static str,
    ) -> Result<(), MemoryError> {
        if values.is_empty() {
            return Ok(());
        }
        let mut data = lock.write();
        let start_u = start as usize;
        let end = start_u
            .checked_add(values.len())
            .ok_or(MemoryError::Overflow)?;
        if end > data.len() {
            return Err(MemoryError::OutOfRange { area, addr: start });
        }
        data[start_u..end].copy_from_slice(values);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("address out of range in {area}: {addr}")]
    OutOfRange { area: &'static str, addr: u16 },
    #[error("address arithmetic overflow")]
    Overflow,
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
}
