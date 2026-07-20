//! =============================================================================
//! Modbus visibility map — which PLC bits/registers are exposed on Modbus TCP.
//! TIA-style device configuration: export tags to FC1/2/3/4 address spaces.
//! =============================================================================

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::compiler::MemArea;

/// Modbus table (function-code area).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModbusTable {
    /// FC01 / 05 / 15 — coils
    Coil,
    /// FC02 — discrete inputs
    Discrete,
    /// FC03 / 06 / 16 — holding registers
    Holding,
    /// FC04 — input registers
    InputReg,
}

/// One export rule: PLC location → Modbus address (when enabled).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusMapEntry {
    pub id: String,
    pub enabled: bool,
    /// Optional link to PLC tag name
    #[serde(default)]
    pub symbol_name: String,
    pub plc_area: MemArea,
    pub plc_index: u16,
    pub modbus_table: ModbusTable,
    pub modbus_address: u16,
    #[serde(default)]
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusMapSnapshot {
    pub entries: Vec<ModbusMapEntry>,
    /// When true, unmapped addresses fall back to identity (addr → same PLC index)
    pub identity_fallback: bool,
}

/// Runtime map with fast lookup: (table, modbus_addr) → (plc_area, plc_index)
pub struct ModbusMap {
    entries: RwLock<Vec<ModbusMapEntry>>,
    identity_fallback: RwLock<bool>,
}

impl ModbusMap {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(default_map()),
            identity_fallback: RwLock::new(true),
        })
    }

    pub fn snapshot(&self) -> ModbusMapSnapshot {
        ModbusMapSnapshot {
            entries: self.entries.read().clone(),
            identity_fallback: *self.identity_fallback.read(),
        }
    }

    pub fn set_all(&self, snap: ModbusMapSnapshot) {
        *self.entries.write() = snap.entries;
        *self.identity_fallback.write() = snap.identity_fallback;
    }

    pub fn identity_fallback(&self) -> bool {
        *self.identity_fallback.read()
    }

    /// Resolve Modbus coil/discrete address to PLC bit (area, index).
    pub fn resolve_bit(
        &self,
        table: ModbusTable,
        modbus_addr: u16,
    ) -> Option<(MemArea, u16)> {
        let entries = self.entries.read();
        for e in entries.iter() {
            if e.enabled && e.modbus_table == table && e.modbus_address == modbus_addr {
                return Some((e.plc_area, e.plc_index));
            }
        }
        if *self.identity_fallback.read() {
            let area = match table {
                ModbusTable::Coil => MemArea::Coil,
                ModbusTable::Discrete => MemArea::Discrete,
                ModbusTable::Holding | ModbusTable::InputReg => return None,
            };
            return Some((area, modbus_addr));
        }
        None
    }

    /// Resolve Modbus register address to PLC word.
    pub fn resolve_reg(
        &self,
        table: ModbusTable,
        modbus_addr: u16,
    ) -> Option<(MemArea, u16)> {
        let entries = self.entries.read();
        for e in entries.iter() {
            if e.enabled && e.modbus_table == table && e.modbus_address == modbus_addr {
                return Some((e.plc_area, e.plc_index));
            }
        }
        if *self.identity_fallback.read() {
            let area = match table {
                ModbusTable::Holding => MemArea::Holding,
                ModbusTable::InputReg => MemArea::InputReg,
                ModbusTable::Coil | ModbusTable::Discrete => return None,
            };
            return Some((area, modbus_addr));
        }
        None
    }

    /// Build enabled-address set for range validation (optional).
    pub fn enabled_modbus_addrs(&self, table: ModbusTable) -> HashMap<u16, (MemArea, u16)> {
        let mut m = HashMap::new();
        for e in self.entries.read().iter() {
            if e.enabled && e.modbus_table == table {
                m.insert(e.modbus_address, (e.plc_area, e.plc_index));
            }
        }
        m
    }
}

fn default_map() -> Vec<ModbusMapEntry> {
    let mut v = Vec::new();
    // Export first 16 I/Q and key holding registers
    for i in 0..16u16 {
        v.push(ModbusMapEntry {
            id: format!("map_i{i}"),
            enabled: true,
            symbol_name: if i == 0 {
                "Start_PB".into()
            } else {
                String::new()
            },
            plc_area: MemArea::Discrete,
            plc_index: i,
            modbus_table: ModbusTable::Discrete,
            modbus_address: i,
            comment: format!("I{i} → DI{i}"),
        });
        v.push(ModbusMapEntry {
            id: format!("map_q{i}"),
            enabled: true,
            symbol_name: if i == 0 {
                "Motor_Run".into()
            } else {
                String::new()
            },
            plc_area: MemArea::Coil,
            plc_index: i,
            modbus_table: ModbusTable::Coil,
            modbus_address: i,
            comment: format!("Q{i} → Coil{i}"),
        });
    }
    for i in [0u16, 1, 40, 41, 42] {
        v.push(ModbusMapEntry {
            id: format!("map_mw{i}"),
            enabled: true,
            symbol_name: String::new(),
            plc_area: MemArea::Holding,
            plc_index: i,
            modbus_table: ModbusTable::Holding,
            modbus_address: i,
            comment: format!("MW{i} → HR{i}"),
        });
    }
    for i in 0..8u16 {
        v.push(ModbusMapEntry {
            id: format!("map_ir{i}"),
            enabled: true,
            symbol_name: String::new(),
            plc_area: MemArea::InputReg,
            plc_index: i,
            modbus_table: ModbusTable::InputReg,
            modbus_address: i,
            comment: format!("IW{i} → IR{i}"),
        });
    }
    v
}
