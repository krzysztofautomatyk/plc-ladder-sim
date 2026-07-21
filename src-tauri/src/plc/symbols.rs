//! =============================================================================
//! PLC Symbol / Tag table (TIA Portal style PLC tags).
//! Defines named bits and registers for ladder addressing and Modbus export.
//! =============================================================================

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::compiler::MemArea;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Bool,
    Word,
    Int,
    DInt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlcSymbol {
    pub id: String,
    /// Tag name e.g. "Start_PB", "Motor_Run"
    pub name: String,
    pub area: MemArea,
    pub index: u16,
    pub data_type: DataType,
    #[serde(default)]
    pub comment: String,
    /// Absolute address display e.g. I0.0 / Q0.0 / MW10 (UI helper)
    #[serde(default)]
    pub address_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTableSnapshot {
    pub symbols: Vec<PlcSymbol>,
}

pub struct SymbolTable {
    symbols: RwLock<Vec<PlcSymbol>>,
}

impl SymbolTable {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            symbols: RwLock::new(default_symbols()),
        })
    }

    pub fn list(&self) -> Vec<PlcSymbol> {
        self.symbols.read().clone()
    }

    pub fn set_all(&self, symbols: Vec<PlcSymbol>) {
        *self.symbols.write() = symbols;
    }

    pub fn upsert(&self, symbol: PlcSymbol) {
        let mut list = self.symbols.write();
        if let Some(i) = list
            .iter()
            .position(|s| s.id == symbol.id || s.name == symbol.name)
        {
            list[i] = symbol;
        } else {
            list.push(symbol);
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        let mut list = self.symbols.write();
        let before = list.len();
        list.retain(|s| s.id != id);
        list.len() < before
    }

    pub fn snapshot(&self) -> SymbolTableSnapshot {
        SymbolTableSnapshot {
            symbols: self.list(),
        }
    }
}

fn default_symbols() -> Vec<PlcSymbol> {
    vec![
        sym(
            "s_i0",
            "Start_PB",
            MemArea::Discrete,
            0,
            DataType::Bool,
            "Start pushbutton",
        ),
        sym(
            "s_i1",
            "Stop_PB",
            MemArea::Discrete,
            1,
            DataType::Bool,
            "Stop pushbutton (NC logic)",
        ),
        sym(
            "s_i2",
            "Count_Pulse",
            MemArea::Discrete,
            2,
            DataType::Bool,
            "Counter pulse",
        ),
        sym(
            "s_i3",
            "Count_Reset",
            MemArea::Discrete,
            3,
            DataType::Bool,
            "Counter reset",
        ),
        sym(
            "s_q0",
            "Motor_Run",
            MemArea::Coil,
            0,
            DataType::Bool,
            "Motor run output",
        ),
        sym(
            "s_q1",
            "Delay_Done",
            MemArea::Coil,
            1,
            DataType::Bool,
            "TON done",
        ),
        sym(
            "s_q2",
            "Count_Done",
            MemArea::Coil,
            2,
            DataType::Bool,
            "CTU done",
        ),
        sym(
            "s_q3",
            "Cmp_Ok",
            MemArea::Coil,
            3,
            DataType::Bool,
            "Compare result",
        ),
        sym(
            "s_mw0",
            "Timer_ET",
            MemArea::Holding,
            0,
            DataType::Word,
            "Timer elapsed (ms)",
        ),
        sym(
            "s_mw40",
            "Setpoint",
            MemArea::Holding,
            40,
            DataType::Word,
            "Compare A",
        ),
        sym(
            "s_mw41",
            "Actual",
            MemArea::Holding,
            41,
            DataType::Word,
            "Compare B",
        ),
        sym(
            "s_mw42",
            "Result",
            MemArea::Holding,
            42,
            DataType::Word,
            "MOVE destination",
        ),
    ]
}

fn sym(
    id: &str,
    name: &str,
    area: MemArea,
    index: u16,
    data_type: DataType,
    comment: &str,
) -> PlcSymbol {
    let address_display = match area {
        MemArea::Discrete => format!("I{}", index),
        MemArea::Coil => format!("Q{}", index),
        MemArea::Holding => format!("MW{}", index),
        MemArea::InputReg => format!("IW{}", index),
        MemArea::MemoryBit => format!("M{}", index),
        MemArea::MemoryWord => format!("MR{}", index),
    };
    PlcSymbol {
        id: id.into(),
        name: name.into(),
        area,
        index,
        data_type,
        comment: comment.into(),
        address_display,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_populated() {
        let t = SymbolTable::new();
        assert!(!t.list().is_empty());
        assert_eq!(t.snapshot().symbols.len(), t.list().len());
    }

    #[test]
    fn upsert_updates_in_place_by_id_then_by_name() {
        let t = SymbolTable::new();
        t.set_all(vec![]);
        assert!(t.list().is_empty());

        let s = sym("s1", "Motor", MemArea::Coil, 0, DataType::Bool, "");
        t.upsert(s.clone());
        assert_eq!(t.list().len(), 1);

        // Same id → replace in place.
        let mut renamed = s.clone();
        renamed.name = "Motor_Run".into();
        t.upsert(renamed);
        assert_eq!(t.list().len(), 1);
        assert_eq!(t.list()[0].name, "Motor_Run");

        // Different id but matching name → still replaces (name is unique key too).
        let by_name = sym("s2", "Motor_Run", MemArea::Coil, 7, DataType::Bool, "");
        t.upsert(by_name);
        assert_eq!(t.list().len(), 1);
        assert_eq!(t.list()[0].index, 7);
    }

    #[test]
    fn remove_reports_whether_anything_was_deleted() {
        let t = SymbolTable::new();
        t.set_all(vec![sym(
            "s1",
            "Tag",
            MemArea::Holding,
            3,
            DataType::Word,
            "",
        )]);
        assert!(t.remove("s1"));
        assert!(t.list().is_empty());
        assert!(!t.remove("missing"));
    }
}
