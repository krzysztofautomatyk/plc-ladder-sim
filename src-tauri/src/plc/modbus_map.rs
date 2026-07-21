//! =============================================================================
//! Modbus Translation Matrix — enterprise mapping engine.
//!
//! Bidirectional translator between PLC process image and Modbus tables:
//!   Direct (1:1 bit/word), BitToRegister (16 bits → 1 reg), RegisterToBit
//!   (1 reg → 16 coils/DI), with per-rule write protection and a global
//!   Strict / SilentDrop deny mode.
//!
//! Internal areas (MemoryBit / MemoryWord) are only reachable via an *enabled*
//! mapping rule — never via identity fallback.
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

/// How a rule maps PLC memory to a Modbus table entry/range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MappingType {
    /// Bit↔bit or word↔word (one Modbus address ↔ one PLC address).
    #[default]
    Direct,
    /// 16 consecutive PLC bits → one 16-bit Modbus register (and reverse on write).
    BitToRegister,
    /// One 16-bit PLC register → 16 consecutive Modbus coils / discrete inputs.
    RegisterToBit,
}

/// Behaviour when a write is denied (per-rule protect or global write off for
/// protected-path handling). Global write-disabled still uses ServerDeviceFailure
/// at the slave layer; per-rule protect uses this mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WriteProtectMode {
    /// Return Modbus exception `IllegalDataAddress` (0x02).
    #[default]
    Strict,
    /// Acknowledge the write but leave PLC memory unchanged.
    SilentDrop,
}

/// One export / translation rule in the matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusMapEntry {
    pub id: String,
    pub enabled: bool,
    #[serde(default)]
    pub mapping_type: MappingType,
    /// Optional link to PLC tag name
    #[serde(default)]
    pub symbol_name: String,
    pub plc_area: MemArea,
    /// PLC start index (bit or word depending on mapping type).
    #[serde(alias = "plc_index")]
    pub plc_start: u16,
    /// Bit offset inside a PLC word (RegisterToBit / packing). 0–15.
    #[serde(default)]
    pub plc_bit_offset: u8,
    pub modbus_table: ModbusTable,
    /// First Modbus address covered by this rule.
    #[serde(alias = "modbus_address")]
    pub modbus_start: u16,
    /// Span on the Modbus side (Direct: 1; BitToRegister: 1 reg; RegisterToBit: 16).
    #[serde(default = "default_length")]
    pub length: u16,
    /// When true, Modbus writes targeting this rule are denied (see WriteProtectMode).
    #[serde(default)]
    pub is_write_protected: bool,
    #[serde(default)]
    pub comment: String,
}

fn default_length() -> u16 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusMapSnapshot {
    pub entries: Vec<ModbusMapEntry>,
    /// When true, unmapped addresses fall back to identity (addr → same PLC index)
    /// for the canonical process-image areas only (never M/MR).
    #[serde(default = "default_true")]
    pub identity_fallback: bool,
    #[serde(default)]
    pub write_protect_mode: WriteProtectMode,
}

fn default_true() -> bool {
    true
}

// ─── Hot-path resolution ─────────────────────────────────────────────────────

/// Resolved Modbus coil/DI address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedBit {
    /// Direct bit in a bit-area (Coil / Discrete / MemoryBit).
    Direct {
        area: MemArea,
        index: u16,
        write_protected: bool,
    },
    /// One bit extracted from a PLC word (RegisterToBit).
    FromWord {
        area: MemArea,
        word_index: u16,
        bit: u8,
        write_protected: bool,
    },
}

impl ResolvedBit {
    pub fn write_protected(self) -> bool {
        match self {
            Self::Direct {
                write_protected, ..
            }
            | Self::FromWord {
                write_protected, ..
            } => write_protected,
        }
    }
}

/// Resolved Modbus holding/input register address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedReg {
    /// Direct word in a word-area (Holding / InputReg / MemoryWord).
    Direct {
        area: MemArea,
        index: u16,
        write_protected: bool,
    },
    /// Word assembled from 16 consecutive PLC bits (BitToRegister).
    FromBits {
        area: MemArea,
        start: u16,
        write_protected: bool,
    },
}

impl ResolvedReg {
    pub fn write_protected(self) -> bool {
        match self {
            Self::Direct {
                write_protected, ..
            }
            | Self::FromBits {
                write_protected, ..
            } => write_protected,
        }
    }
}

/// Runtime map with O(1) lookup after compile.
pub struct ModbusMap {
    entries: RwLock<Vec<ModbusMapEntry>>,
    identity_fallback: RwLock<bool>,
    write_protect_mode: RwLock<WriteProtectMode>,
    bit_index: RwLock<HashMap<(ModbusTable, u16), ResolvedBit>>,
    reg_index: RwLock<HashMap<(ModbusTable, u16), ResolvedReg>>,
}

impl ModbusMap {
    pub fn new() -> Arc<Self> {
        let entries = default_map();
        let (bit_index, reg_index) = compile_indices(&entries).expect("default map is valid");
        Arc::new(Self {
            entries: RwLock::new(entries),
            identity_fallback: RwLock::new(true),
            write_protect_mode: RwLock::new(WriteProtectMode::Strict),
            bit_index: RwLock::new(bit_index),
            reg_index: RwLock::new(reg_index),
        })
    }

    pub fn snapshot(&self) -> ModbusMapSnapshot {
        ModbusMapSnapshot {
            entries: self.entries.read().clone(),
            identity_fallback: *self.identity_fallback.read(),
            write_protect_mode: *self.write_protect_mode.read(),
        }
    }

    pub fn write_protect_mode(&self) -> WriteProtectMode {
        *self.write_protect_mode.read()
    }

    /// Validate and install a new snapshot. On error the previous map is kept.
    pub fn set_all(&self, snap: ModbusMapSnapshot) -> Result<(), String> {
        validate_snapshot(&snap)?;
        let (bit_index, reg_index) = compile_indices(&snap.entries)?;
        *self.entries.write() = snap.entries;
        *self.identity_fallback.write() = snap.identity_fallback;
        *self.write_protect_mode.write() = snap.write_protect_mode;
        *self.bit_index.write() = bit_index;
        *self.reg_index.write() = reg_index;
        Ok(())
    }

    /// Resolve a Modbus coil / discrete address.
    pub fn resolve_bit(&self, table: ModbusTable, modbus_addr: u16) -> Option<ResolvedBit> {
        if let Some(r) = self.bit_index.read().get(&(table, modbus_addr)).copied() {
            return Some(r);
        }
        if *self.identity_fallback.read() {
            let area = match table {
                ModbusTable::Coil => MemArea::Coil,
                ModbusTable::Discrete => MemArea::Discrete,
                ModbusTable::Holding | ModbusTable::InputReg => return None,
            };
            return Some(ResolvedBit::Direct {
                area,
                index: modbus_addr,
                write_protected: false,
            });
        }
        None
    }

    /// Resolve a Modbus holding / input register address.
    pub fn resolve_reg(&self, table: ModbusTable, modbus_addr: u16) -> Option<ResolvedReg> {
        if let Some(r) = self.reg_index.read().get(&(table, modbus_addr)).copied() {
            return Some(r);
        }
        if *self.identity_fallback.read() {
            let area = match table {
                ModbusTable::Holding => MemArea::Holding,
                ModbusTable::InputReg => MemArea::InputReg,
                ModbusTable::Coil | ModbusTable::Discrete => return None,
            };
            return Some(ResolvedReg::Direct {
                area,
                index: modbus_addr,
                write_protected: false,
            });
        }
        None
    }
}

// ─── Validation & index compile ──────────────────────────────────────────────

fn is_bit_area(a: MemArea) -> bool {
    matches!(a, MemArea::Coil | MemArea::Discrete | MemArea::MemoryBit)
}

fn is_word_area(a: MemArea) -> bool {
    matches!(
        a,
        MemArea::Holding | MemArea::InputReg | MemArea::MemoryWord
    )
}

fn is_bit_table(t: ModbusTable) -> bool {
    matches!(t, ModbusTable::Coil | ModbusTable::Discrete)
}

fn is_reg_table(t: ModbusTable) -> bool {
    matches!(t, ModbusTable::Holding | ModbusTable::InputReg)
}

/// Validate a full snapshot before install.
pub fn validate_snapshot(snap: &ModbusMapSnapshot) -> Result<(), String> {
    for (i, e) in snap.entries.iter().enumerate() {
        validate_entry(e).map_err(|msg| format!("entry[{i}] id={}: {msg}", e.id))?;
    }
    // Overlap check on enabled rules only (compile_indices also enforces this).
    compile_indices(&snap.entries).map(|_| ())
}

fn validate_entry(e: &ModbusMapEntry) -> Result<(), String> {
    if e.plc_bit_offset > 15 {
        return Err(format!(
            "plc_bit_offset {} out of range 0–15",
            e.plc_bit_offset
        ));
    }
    match e.mapping_type {
        MappingType::Direct => {
            let bit_plc = is_bit_area(e.plc_area);
            let word_plc = is_word_area(e.plc_area);
            let bit_mb = is_bit_table(e.modbus_table);
            let reg_mb = is_reg_table(e.modbus_table);
            if bit_plc && bit_mb {
                // ok bit↔bit
            } else if word_plc && reg_mb {
                // ok word↔word
            } else {
                return Err(format!(
                    "Direct mapping requires bit↔bit or word↔word (plc={:?}, mb={:?})",
                    e.plc_area, e.modbus_table
                ));
            }
            if e.length == 0 {
                return Err("Direct length must be ≥ 1".into());
            }
        }
        MappingType::BitToRegister => {
            if !is_bit_area(e.plc_area) {
                return Err(format!(
                    "BitToRegister requires PLC bit area (got {:?})",
                    e.plc_area
                ));
            }
            if !is_reg_table(e.modbus_table) {
                return Err(format!(
                    "BitToRegister requires Modbus register table (got {:?})",
                    e.modbus_table
                ));
            }
            // Need 16 consecutive PLC bits starting at plc_start.
            if e.plc_start.checked_add(15).is_none() {
                return Err("BitToRegister plc_start overflows u16 for 16-bit span".into());
            }
        }
        MappingType::RegisterToBit => {
            if !is_word_area(e.plc_area) {
                return Err(format!(
                    "RegisterToBit requires PLC word area (got {:?})",
                    e.plc_area
                ));
            }
            if !is_bit_table(e.modbus_table) {
                return Err(format!(
                    "RegisterToBit requires Modbus coil/discrete table (got {:?})",
                    e.modbus_table
                ));
            }
            if e.modbus_start.checked_add(15).is_none() {
                return Err("RegisterToBit modbus_start overflows u16 for 16-bit span".into());
            }
            // Full 16-bit word export uses bits plc_bit_offset .. +15; only offset 0 fits.
            if e.plc_bit_offset != 0 {
                return Err(
                    "RegisterToBit requires plc_bit_offset=0 (exports all 16 bits of the word)"
                        .into(),
                );
            }
        }
    }
    Ok(())
}

type BitIndexMap = HashMap<(ModbusTable, u16), ResolvedBit>;
type RegIndexMap = HashMap<(ModbusTable, u16), ResolvedReg>;

fn compile_indices(entries: &[ModbusMapEntry]) -> Result<(BitIndexMap, RegIndexMap), String> {
    let mut bit_index: BitIndexMap = HashMap::new();
    let mut reg_index: RegIndexMap = HashMap::new();

    let insert_bit = |idx: &mut HashMap<(ModbusTable, u16), ResolvedBit>,
                      table: ModbusTable,
                      addr: u16,
                      r: ResolvedBit|
     -> Result<(), String> {
        if idx.insert((table, addr), r).is_some() {
            Err(format!(
                "overlapping Modbus bit mapping on {:?}:{addr}",
                table
            ))
        } else {
            Ok(())
        }
    };
    let insert_reg = |idx: &mut HashMap<(ModbusTable, u16), ResolvedReg>,
                      table: ModbusTable,
                      addr: u16,
                      r: ResolvedReg|
     -> Result<(), String> {
        if idx.insert((table, addr), r).is_some() {
            Err(format!(
                "overlapping Modbus register mapping on {:?}:{addr}",
                table
            ))
        } else {
            Ok(())
        }
    };

    for e in entries.iter().filter(|e| e.enabled) {
        validate_entry(e)?;
        let wp = e.is_write_protected;
        match e.mapping_type {
            MappingType::Direct => {
                if is_bit_table(e.modbus_table) {
                    let n = e.length.max(1);
                    for i in 0..n {
                        let mb = e
                            .modbus_start
                            .checked_add(i)
                            .ok_or_else(|| "modbus_start+length overflow".to_string())?;
                        let plc = e
                            .plc_start
                            .checked_add(i)
                            .ok_or_else(|| "plc_start+length overflow".to_string())?;
                        insert_bit(
                            &mut bit_index,
                            e.modbus_table,
                            mb,
                            ResolvedBit::Direct {
                                area: e.plc_area,
                                index: plc,
                                write_protected: wp,
                            },
                        )?;
                    }
                } else {
                    let n = e.length.max(1);
                    for i in 0..n {
                        let mb = e
                            .modbus_start
                            .checked_add(i)
                            .ok_or_else(|| "modbus_start+length overflow".to_string())?;
                        let plc = e
                            .plc_start
                            .checked_add(i)
                            .ok_or_else(|| "plc_start+length overflow".to_string())?;
                        insert_reg(
                            &mut reg_index,
                            e.modbus_table,
                            mb,
                            ResolvedReg::Direct {
                                area: e.plc_area,
                                index: plc,
                                write_protected: wp,
                            },
                        )?;
                    }
                }
            }
            MappingType::BitToRegister => {
                // One Modbus register packing 16 PLC bits starting at plc_start.
                insert_reg(
                    &mut reg_index,
                    e.modbus_table,
                    e.modbus_start,
                    ResolvedReg::FromBits {
                        area: e.plc_area,
                        start: e.plc_start,
                        write_protected: wp,
                    },
                )?;
            }
            MappingType::RegisterToBit => {
                // 16 Modbus bits from one PLC word; bit i uses word bit (plc_bit_offset + i).
                for i in 0u16..16 {
                    let mb = e.modbus_start.saturating_add(i);
                    let bit = e.plc_bit_offset.saturating_add(i as u8);
                    if bit > 15 {
                        break;
                    }
                    insert_bit(
                        &mut bit_index,
                        e.modbus_table,
                        mb,
                        ResolvedBit::FromWord {
                            area: e.plc_area,
                            word_index: e.plc_start,
                            bit,
                            write_protected: wp,
                        },
                    )?;
                }
            }
        }
    }

    Ok((bit_index, reg_index))
}

fn default_map() -> Vec<ModbusMapEntry> {
    let mut v = Vec::new();
    // Direct: first 16 I/Q (identity-style) — I write-protected (physical inputs).
    for i in 0..16u16 {
        v.push(ModbusMapEntry {
            id: format!("map_i{i}"),
            enabled: true,
            mapping_type: MappingType::Direct,
            symbol_name: if i == 0 {
                "Start_PB".into()
            } else {
                String::new()
            },
            plc_area: MemArea::Discrete,
            plc_start: i,
            plc_bit_offset: 0,
            modbus_table: ModbusTable::Discrete,
            modbus_start: i,
            length: 1,
            is_write_protected: true,
            comment: format!("I{i} → DI{i} (RO)"),
        });
        v.push(ModbusMapEntry {
            id: format!("map_q{i}"),
            enabled: true,
            mapping_type: MappingType::Direct,
            symbol_name: if i == 0 {
                "Motor_Run".into()
            } else {
                String::new()
            },
            plc_area: MemArea::Coil,
            plc_start: i,
            plc_bit_offset: 0,
            modbus_table: ModbusTable::Coil,
            modbus_start: i,
            length: 1,
            is_write_protected: false,
            comment: format!("Q{i} → Coil{i}"),
        });
    }
    // Key holding registers + reserved setpoint (write-protected).
    for i in [0u16, 1, 40, 41, 42] {
        v.push(ModbusMapEntry {
            id: format!("map_mw{i}"),
            enabled: true,
            mapping_type: MappingType::Direct,
            symbol_name: String::new(),
            plc_area: MemArea::Holding,
            plc_start: i,
            plc_bit_offset: 0,
            modbus_table: ModbusTable::Holding,
            modbus_start: i,
            length: 1,
            is_write_protected: false,
            comment: format!("R{i} → HR{i}"),
        });
    }
    v.push(ModbusMapEntry {
        id: "map_r500_wp".into(),
        enabled: true,
        mapping_type: MappingType::Direct,
        symbol_name: String::new(),
        plc_area: MemArea::Holding,
        plc_start: 500,
        plc_bit_offset: 0,
        modbus_table: ModbusTable::Holding,
        modbus_start: 500,
        length: 1,
        is_write_protected: true,
        comment: "R500 reserved setpoint (write-protected)".into(),
    });
    // BitToRegister: M0–M15 → HR100
    v.push(ModbusMapEntry {
        id: "map_m0_15_hr100".into(),
        enabled: true,
        mapping_type: MappingType::BitToRegister,
        symbol_name: String::new(),
        plc_area: MemArea::MemoryBit,
        plc_start: 0,
        plc_bit_offset: 0,
        modbus_table: ModbusTable::Holding,
        modbus_start: 100,
        length: 1,
        is_write_protected: false,
        comment: "M0–M15 packed → HR100".into(),
    });
    // RegisterToBit: R10 → coils 101–116 (write-protected status bits)
    v.push(ModbusMapEntry {
        id: "map_r10_coils101".into(),
        enabled: true,
        mapping_type: MappingType::RegisterToBit,
        symbol_name: String::new(),
        plc_area: MemArea::Holding,
        plc_start: 10,
        plc_bit_offset: 0,
        modbus_table: ModbusTable::Coil,
        modbus_start: 101,
        length: 16,
        is_write_protected: true,
        comment: "R10 bits → Coil101–116 (RO)".into(),
    });
    // Direct: M100 → Coil 200 (remote AUTO — SCADA writable when global writes on)
    v.push(ModbusMapEntry {
        id: "map_m100_coil200".into(),
        enabled: true,
        mapping_type: MappingType::Direct,
        symbol_name: String::new(),
        plc_area: MemArea::MemoryBit,
        plc_start: 100,
        plc_bit_offset: 0,
        modbus_table: ModbusTable::Coil,
        modbus_start: 200,
        length: 1,
        is_write_protected: false,
        comment: "M100 → Coil200 remote AUTO".into(),
    });
    for i in 0..8u16 {
        v.push(ModbusMapEntry {
            id: format!("map_ir{i}"),
            enabled: true,
            mapping_type: MappingType::Direct,
            symbol_name: String::new(),
            plc_area: MemArea::InputReg,
            plc_start: i,
            plc_bit_offset: 0,
            modbus_table: ModbusTable::InputReg,
            modbus_start: i,
            length: 1,
            is_write_protected: true,
            comment: format!("IW{i} → IR{i} (RO)"),
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_snap(entries: Vec<ModbusMapEntry>) -> ModbusMapSnapshot {
        ModbusMapSnapshot {
            entries,
            identity_fallback: false,
            write_protect_mode: WriteProtectMode::Strict,
        }
    }

    fn direct_bit(
        id: &str,
        plc: MemArea,
        plc_i: u16,
        table: ModbusTable,
        mb: u16,
        wp: bool,
    ) -> ModbusMapEntry {
        ModbusMapEntry {
            id: id.into(),
            enabled: true,
            mapping_type: MappingType::Direct,
            symbol_name: String::new(),
            plc_area: plc,
            plc_start: plc_i,
            plc_bit_offset: 0,
            modbus_table: table,
            modbus_start: mb,
            length: 1,
            is_write_protected: wp,
            comment: String::new(),
        }
    }

    #[test]
    fn default_map_is_valid() {
        let m = ModbusMap::new();
        let snap = m.snapshot();
        assert!(!snap.entries.is_empty());
        assert!(validate_snapshot(&snap).is_ok());
    }

    #[test]
    fn rejects_illegal_direct_cross_kind() {
        let e = direct_bit("bad", MemArea::Holding, 0, ModbusTable::Coil, 0, false);
        assert!(validate_entry(&e).is_err());
    }

    #[test]
    fn rejects_overlapping_modbus_addresses() {
        let snap = empty_snap(vec![
            direct_bit("a", MemArea::Coil, 0, ModbusTable::Coil, 5, false),
            direct_bit("b", MemArea::Coil, 1, ModbusTable::Coil, 5, false),
        ]);
        assert!(validate_snapshot(&snap).is_err());
    }

    #[test]
    fn bit_to_register_and_register_to_bit_compile() {
        let snap = empty_snap(vec![
            ModbusMapEntry {
                id: "b2r".into(),
                enabled: true,
                mapping_type: MappingType::BitToRegister,
                symbol_name: String::new(),
                plc_area: MemArea::MemoryBit,
                plc_start: 0,
                plc_bit_offset: 0,
                modbus_table: ModbusTable::Holding,
                modbus_start: 100,
                length: 1,
                is_write_protected: false,
                comment: String::new(),
            },
            ModbusMapEntry {
                id: "r2b".into(),
                enabled: true,
                mapping_type: MappingType::RegisterToBit,
                symbol_name: String::new(),
                plc_area: MemArea::Holding,
                plc_start: 10,
                plc_bit_offset: 0,
                modbus_table: ModbusTable::Coil,
                modbus_start: 101,
                length: 16,
                is_write_protected: true,
                comment: String::new(),
            },
        ]);
        let m = ModbusMap::new();
        m.set_all(snap).unwrap();

        match m.resolve_reg(ModbusTable::Holding, 100).unwrap() {
            ResolvedReg::FromBits { area, start, .. } => {
                assert_eq!(area, MemArea::MemoryBit);
                assert_eq!(start, 0);
            }
            other => panic!("expected FromBits, got {other:?}"),
        }
        match m.resolve_bit(ModbusTable::Coil, 101).unwrap() {
            ResolvedBit::FromWord {
                word_index, bit, ..
            } => {
                assert_eq!(word_index, 10);
                assert_eq!(bit, 0);
            }
            other => panic!("expected FromWord, got {other:?}"),
        }
        match m.resolve_bit(ModbusTable::Coil, 116).unwrap() {
            ResolvedBit::FromWord {
                bit,
                write_protected,
                ..
            } => {
                assert_eq!(bit, 15);
                assert!(write_protected);
            }
            other => panic!("expected FromWord, got {other:?}"),
        }
    }

    #[test]
    fn identity_fallback_never_exposes_memory_bit() {
        let m = ModbusMap::new();
        // Clear custom rules; keep identity on.
        m.set_all(ModbusMapSnapshot {
            entries: vec![],
            identity_fallback: true,
            write_protect_mode: WriteProtectMode::Strict,
        })
        .unwrap();
        // Coil 0 → Coil 0 via identity, not MemoryBit.
        match m.resolve_bit(ModbusTable::Coil, 0).unwrap() {
            ResolvedBit::Direct { area, .. } => assert_eq!(area, MemArea::Coil),
            other => panic!("{other:?}"),
        }
        // No identity path for inventing M from holding table bits.
        assert!(m.resolve_bit(ModbusTable::Holding, 0).is_none());
    }

    #[test]
    fn serde_accepts_legacy_field_names() {
        let json = r#"{
            "entries": [{
                "id": "legacy",
                "enabled": true,
                "symbol_name": "",
                "plc_area": "coil",
                "plc_index": 3,
                "modbus_table": "coil",
                "modbus_address": 7,
                "comment": "old"
            }],
            "identity_fallback": false
        }"#;
        let snap: ModbusMapSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(snap.entries[0].plc_start, 3);
        assert_eq!(snap.entries[0].modbus_start, 7);
        assert_eq!(snap.entries[0].mapping_type, MappingType::Direct);
        assert!(!snap.entries[0].is_write_protected);
        assert_eq!(snap.write_protect_mode, WriteProtectMode::Strict);
        validate_snapshot(&snap).unwrap();
    }

    #[test]
    fn set_all_rejects_invalid_keeps_previous() {
        let m = ModbusMap::new();
        let before = m.snapshot().entries.len();
        let bad = empty_snap(vec![direct_bit(
            "x",
            MemArea::Holding,
            0,
            ModbusTable::Coil,
            0,
            false,
        )]);
        assert!(m.set_all(bad).is_err());
        assert_eq!(m.snapshot().entries.len(), before);
    }
}
