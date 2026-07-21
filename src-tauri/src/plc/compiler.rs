//! =============================================================================
//! Ladder AST → bytecode compiler.
//! Elements: NO/NC, coils, TON/TOF/RTO, CTU/CTD, MOVE, compare, math.
//! Parallel OR branches: or_branches merge then series `elements` continue.
//! =============================================================================

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Memory area addressing (IEC-like).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemArea {
    Coil,
    Discrete,
    Holding,
    InputReg,
    /// Internal marker bit (M) — usable in ladder logic only, never on Modbus.
    MemoryBit,
    /// Internal memory register (MR) — usable in ladder logic only, never on Modbus.
    MemoryWord,
}

/// Address reference used by contacts, coils, and function blocks.
/// Optional `bit` (0–15) selects a bit inside a word (e.g. R1.3 → holding 1 bit 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub area: MemArea,
    pub index: u16,
    /// Bit number within a word register (Modbus/holding style). None = whole word or discrete coil bit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit: Option<u8>,
}

impl Address {
    pub fn coil(index: u16) -> Self {
        Self {
            area: MemArea::Coil,
            index,
            bit: None,
        }
    }
    pub fn discrete(index: u16) -> Self {
        Self {
            area: MemArea::Discrete,
            index,
            bit: None,
        }
    }
    pub fn holding(index: u16) -> Self {
        Self {
            area: MemArea::Holding,
            index,
            bit: None,
        }
    }
}

/// Compare operators (power rail TRUE when relation holds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmpOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

/// Single ladder element in the visual / AST representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LadderElement {
    ContactNo {
        id: String,
        address: Address,
    },
    ContactNc {
        id: String,
        address: Address,
    },
    /// Rising edge (positive transition / P) — TRUE one scan on 0→1
    ContactRising {
        id: String,
        address: Address,
    },
    /// Falling edge (negative transition / N) — TRUE one scan on 1→0
    ContactFalling {
        id: String,
        address: Address,
    },
    Coil {
        id: String,
        address: Address,
    },
    CoilNegated {
        id: String,
        address: Address,
    },
    /// SET / latch — when power TRUE, bit stays 1 until RESET
    CoilSet {
        id: String,
        address: Address,
    },
    /// RESET / unlatch — when power TRUE, bit clears to 0
    CoilReset {
        id: String,
        address: Address,
    },
    Ton {
        id: String,
        preset_ms: u32,
        timer_index: u16,
        done_address: Option<Address>,
    },
    Tof {
        id: String,
        preset_ms: u32,
        timer_index: u16,
        done_address: Option<Address>,
    },
    /// Retentive on-delay: ET holds when IN false; reset clears
    Rto {
        id: String,
        preset_ms: u32,
        timer_index: u16,
        done_address: Option<Address>,
        reset_address: Option<Address>,
    },
    Ctu {
        id: String,
        preset: u16,
        counter_index: u16,
        done_address: Option<Address>,
        reset_address: Option<Address>,
    },
    /// Count-down: CV starts at preset, decrements on rising edge of CU
    Ctd {
        id: String,
        preset: u16,
        counter_index: u16,
        done_address: Option<Address>,
        /// Load / reset to preset
        load_address: Option<Address>,
    },
    Math {
        id: String,
        op: MathOp,
        a: Address,
        b: Address,
        dest: Address,
    },
    /// dest := source (word move)
    Move {
        id: String,
        source: Address,
        dest: Address,
    },
    /// Compare a OP b → power rail AND
    Compare {
        id: String,
        op: CmpOp,
        a: Address,
        b: Address,
    },
    Wire {
        id: String,
    },
    /// Parallel OR group placed inline within a rung's series (nestable).
    /// power = incoming AND (branch0 OR branch1 OR …).
    #[serde(rename = "parallel")]
    ParallelGroup {
        id: String,
        branches: Vec<Vec<LadderElement>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// One rung: optional parallel OR network, then series elements (AND + coils).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rung {
    pub id: String,
    pub comment: String,
    /// Series chain after OR merge (or full rung when or_branches empty).
    pub elements: Vec<LadderElement>,
    /// Parallel OR contact networks evaluated before `elements`.
    /// power = OR(each branch series result); then AND with series elements.
    #[serde(default)]
    pub or_branches: Vec<Vec<LadderElement>>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Full ladder program (JSON import/export format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LadderProgram {
    pub name: String,
    pub version: String,
    pub description: String,
    pub rungs: Vec<Rung>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl Default for LadderProgram {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            version: "1.0.0".into(),
            description: String::new(),
            rungs: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// (area, word/bit index, optional bit 0–15 inside holding/input word)
pub type BitRef = (MemArea, u16, Option<u8>);

fn bit_ref(a: &Address) -> BitRef {
    (a.area, a.index, a.bit)
}

/// Bytecode instruction executed by the scan engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instruction {
    LoadNo {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
    },
    LoadNc {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
    },
    /// Positive edge (0→1), one-shot; key = element_id for edge memory
    LoadRising {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
        element_id: String,
    },
    /// Negative edge (1→0), one-shot
    LoadFalling {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
        element_id: String,
    },
    StoreCoil {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
    },
    StoreCoilNegated {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
    },
    /// SET (S) — write 1 when power true; leave unchanged when false
    StoreSet {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
    },
    /// RESET (R) — write 0 when power true; leave unchanged when false
    StoreReset {
        area: MemArea,
        index: u16,
        bit: Option<u8>,
    },
    Ton {
        preset_ms: u32,
        timer_index: u16,
        done: Option<BitRef>,
        element_id: String,
    },
    Tof {
        preset_ms: u32,
        timer_index: u16,
        done: Option<BitRef>,
        element_id: String,
    },
    Rto {
        preset_ms: u32,
        timer_index: u16,
        done: Option<BitRef>,
        reset: Option<BitRef>,
        element_id: String,
    },
    Ctu {
        preset: u16,
        counter_index: u16,
        done: Option<BitRef>,
        reset: Option<BitRef>,
        element_id: String,
    },
    Ctd {
        preset: u16,
        counter_index: u16,
        done: Option<BitRef>,
        load: Option<BitRef>,
        element_id: String,
    },
    Math {
        op: MathOp,
        a: (MemArea, u16),
        b: (MemArea, u16),
        dest: (MemArea, u16),
        element_id: String,
    },
    Move {
        source: (MemArea, u16),
        dest: (MemArea, u16),
        element_id: String,
    },
    Compare {
        op: CmpOp,
        a: (MemArea, u16),
        b: (MemArea, u16),
        element_id: String,
    },
    /// Start OR evaluation (push context)
    OrBegin,
    /// Finish current OR branch, start next
    OrAlt,
    /// Finish OR network: power = any branch true
    OrEnd,
    EndRung {
        rung_id: String,
    },
    Nop,
}

/// Compiled, versioned program with integrity hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledProgram {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub instructions: Vec<Instruction>,
    pub debug_map: BTreeMap<String, usize>,
    pub source: LadderProgram,
}

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("rung {0}: empty (no elements and no OR branches)")]
    EmptyRung(String),
    #[error("rung {0}: invalid element: {1}")]
    InvalidElement(String, String),
    #[error("serialization failed: {0}")]
    Serialize(String),
}

/// Compile ladder AST to bytecode with content hash for traceability.
pub fn compile(program: LadderProgram) -> Result<CompiledProgram, CompileError> {
    if program.rungs.is_empty() {
        let hash = hash_bytes(b"empty");
        return Ok(CompiledProgram {
            name: program.name.clone(),
            version: program.version.clone(),
            hash,
            instructions: Vec::new(),
            debug_map: BTreeMap::new(),
            source: program,
        });
    }

    let mut instructions = Vec::new();
    let mut debug_map = BTreeMap::new();

    for rung in &program.rungs {
        if !rung.enabled {
            continue;
        }
        let has_or = !rung.or_branches.is_empty();
        if !has_or && rung.elements.is_empty() {
            return Err(CompileError::EmptyRung(rung.id.clone()));
        }

        if has_or {
            instructions.push(Instruction::OrBegin);
            for (bi, branch) in rung.or_branches.iter().enumerate() {
                if bi > 0 {
                    instructions.push(Instruction::OrAlt);
                }
                if branch.is_empty() {
                    return Err(CompileError::InvalidElement(
                        rung.id.clone(),
                        format!("empty OR branch {bi}"),
                    ));
                }
                compile_elements(branch, &mut instructions, &mut debug_map)?;
            }
            instructions.push(Instruction::OrEnd);
        }

        compile_elements(&rung.elements, &mut instructions, &mut debug_map)?;

        instructions.push(Instruction::EndRung {
            rung_id: rung.id.clone(),
        });
    }

    let hash = hash_program(&instructions, &program.version, &program.name)?;

    Ok(CompiledProgram {
        name: program.name.clone(),
        version: program.version.clone(),
        hash,
        instructions,
        debug_map,
        source: program,
    })
}

fn compile_elements(
    elements: &[LadderElement],
    instructions: &mut Vec<Instruction>,
    debug_map: &mut BTreeMap<String, usize>,
) -> Result<(), CompileError> {
    for el in elements {
        let idx = instructions.len();
        match el {
            LadderElement::ContactNo { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::LoadNo {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                });
            }
            LadderElement::ContactNc { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::LoadNc {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                });
            }
            LadderElement::ContactRising { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::LoadRising {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                    element_id: id.clone(),
                });
            }
            LadderElement::ContactFalling { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::LoadFalling {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                    element_id: id.clone(),
                });
            }
            LadderElement::Coil { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::StoreCoil {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                });
            }
            LadderElement::CoilNegated { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::StoreCoilNegated {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                });
            }
            LadderElement::CoilSet { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::StoreSet {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                });
            }
            LadderElement::CoilReset { id, address } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::StoreReset {
                    area: address.area,
                    index: address.index,
                    bit: address.bit,
                });
            }
            LadderElement::Ton {
                id,
                preset_ms,
                timer_index,
                done_address,
            } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Ton {
                    preset_ms: *preset_ms,
                    timer_index: *timer_index,
                    done: done_address.as_ref().map(bit_ref),
                    element_id: id.clone(),
                });
            }
            LadderElement::Tof {
                id,
                preset_ms,
                timer_index,
                done_address,
            } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Tof {
                    preset_ms: *preset_ms,
                    timer_index: *timer_index,
                    done: done_address.as_ref().map(bit_ref),
                    element_id: id.clone(),
                });
            }
            LadderElement::Rto {
                id,
                preset_ms,
                timer_index,
                done_address,
                reset_address,
            } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Rto {
                    preset_ms: *preset_ms,
                    timer_index: *timer_index,
                    done: done_address.as_ref().map(bit_ref),
                    reset: reset_address.as_ref().map(bit_ref),
                    element_id: id.clone(),
                });
            }
            LadderElement::Ctu {
                id,
                preset,
                counter_index,
                done_address,
                reset_address,
            } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Ctu {
                    preset: *preset,
                    counter_index: *counter_index,
                    done: done_address.as_ref().map(bit_ref),
                    reset: reset_address.as_ref().map(bit_ref),
                    element_id: id.clone(),
                });
            }
            LadderElement::Ctd {
                id,
                preset,
                counter_index,
                done_address,
                load_address,
            } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Ctd {
                    preset: *preset,
                    counter_index: *counter_index,
                    done: done_address.as_ref().map(bit_ref),
                    load: load_address.as_ref().map(bit_ref),
                    element_id: id.clone(),
                });
            }
            LadderElement::Math { id, op, a, b, dest } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Math {
                    op: *op,
                    a: (a.area, a.index),
                    b: (b.area, b.index),
                    dest: (dest.area, dest.index),
                    element_id: id.clone(),
                });
            }
            LadderElement::Move { id, source, dest } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Move {
                    source: (source.area, source.index),
                    dest: (dest.area, dest.index),
                    element_id: id.clone(),
                });
            }
            LadderElement::Compare { id, op, a, b } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Compare {
                    op: *op,
                    a: (a.area, a.index),
                    b: (b.area, b.index),
                    element_id: id.clone(),
                });
            }
            LadderElement::Wire { id } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::Nop);
            }
            LadderElement::ParallelGroup { id, branches } => {
                debug_map.insert(id.clone(), idx);
                instructions.push(Instruction::OrBegin);
                for (bi, branch) in branches.iter().enumerate() {
                    if bi > 0 {
                        instructions.push(Instruction::OrAlt);
                    }
                    if branch.is_empty() {
                        return Err(CompileError::InvalidElement(
                            id.clone(),
                            format!("empty parallel branch {bi}"),
                        ));
                    }
                    compile_elements(branch, instructions, debug_map)?;
                }
                instructions.push(Instruction::OrEnd);
            }
        }
    }
    Ok(())
}

fn hash_program(
    instructions: &[Instruction],
    version: &str,
    name: &str,
) -> Result<String, CompileError> {
    let payload = bincode::serialize(&(name, version, instructions))
        .map_err(|e| CompileError::Serialize(e.to_string()))?;
    Ok(hash_bytes(&payload))
}

fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BytecodePackage {
    name: String,
    version: String,
    hash: String,
    instructions: Vec<Instruction>,
    debug_map: BTreeMap<String, usize>,
    source_json: String,
}

pub fn export_bytecode(program: &CompiledProgram) -> Result<Vec<u8>, CompileError> {
    let source_json = serde_json::to_string(&program.source)
        .map_err(|e| CompileError::Serialize(e.to_string()))?;
    let pkg = BytecodePackage {
        name: program.name.clone(),
        version: program.version.clone(),
        hash: program.hash.clone(),
        instructions: program.instructions.clone(),
        debug_map: program.debug_map.clone(),
        source_json,
    };
    bincode::serialize(&pkg).map_err(|e| CompileError::Serialize(e.to_string()))
}

pub fn import_bytecode(data: &[u8]) -> Result<CompiledProgram, CompileError> {
    let pkg: BytecodePackage =
        bincode::deserialize(data).map_err(|e| CompileError::Serialize(e.to_string()))?;
    let source: LadderProgram = serde_json::from_str(&pkg.source_json)
        .map_err(|e| CompileError::Serialize(e.to_string()))?;
    Ok(CompiledProgram {
        name: pkg.name,
        version: pkg.version,
        hash: pkg.hash,
        instructions: pkg.instructions,
        debug_map: pkg.debug_map,
        source,
    })
}

/// Demo: classic (I0 OR Q0) AND NOT I1 → Q0, plus TON/CTU/compare/move.
pub fn demo_program() -> LadderProgram {
    LadderProgram {
        name: "Demo_Start_Stop".into(),
        version: "2.0.0".into(),
        description: "OR seal-in, TON, CTU, compare, MOVE demo".into(),
        rungs: vec![
            Rung {
                id: "rung_0".into(),
                comment: "(I0 OR Q0) AND NOT I1 → Q0  [OR branches]".into(),
                enabled: true,
                or_branches: vec![
                    vec![LadderElement::ContactNo {
                        id: "c_i0".into(),
                        address: Address::discrete(0),
                    }],
                    vec![LadderElement::ContactNo {
                        id: "c_q0_seal".into(),
                        address: Address::coil(0),
                    }],
                ],
                elements: vec![
                    LadderElement::ContactNc {
                        id: "c_i1_stop".into(),
                        address: Address::discrete(1),
                    },
                    LadderElement::Coil {
                        id: "coil_q0".into(),
                        address: Address::coil(0),
                    },
                ],
            },
            Rung {
                id: "rung_1".into(),
                comment: "TON 2000ms when Q0 → Q1".into(),
                enabled: true,
                or_branches: vec![],
                elements: vec![
                    LadderElement::ContactNo {
                        id: "c_q0_ton".into(),
                        address: Address::coil(0),
                    },
                    LadderElement::Ton {
                        id: "ton_0".into(),
                        preset_ms: 2000,
                        timer_index: 0,
                        done_address: Some(Address::coil(1)),
                    },
                ],
            },
            Rung {
                id: "rung_2".into(),
                comment: "CTU on I2, preset 5 → Q2; reset I3".into(),
                enabled: true,
                or_branches: vec![],
                elements: vec![
                    LadderElement::ContactNo {
                        id: "c_i2".into(),
                        address: Address::discrete(2),
                    },
                    LadderElement::Ctu {
                        id: "ctu_0".into(),
                        preset: 5,
                        counter_index: 10,
                        done_address: Some(Address::coil(2)),
                        reset_address: Some(Address::discrete(3)),
                    },
                ],
            },
            Rung {
                id: "rung_3".into(),
                // Use MW40+ to avoid collision with timer/counter HR pairs (index*2)
                comment: "If MW40 >= MW41 then MOVE MW40→MW42 and Q3".into(),
                enabled: true,
                or_branches: vec![],
                elements: vec![
                    LadderElement::Compare {
                        id: "cmp_ge".into(),
                        op: CmpOp::Ge,
                        a: Address::holding(40),
                        b: Address::holding(41),
                    },
                    LadderElement::Move {
                        id: "mov_0".into(),
                        source: Address::holding(40),
                        dest: Address::holding(42),
                    },
                    LadderElement::Coil {
                        id: "coil_q3".into(),
                        address: Address::coil(3),
                    },
                ],
            },
        ],
        metadata: {
            let mut m = BTreeMap::new();
            m.insert("author".into(), "system".into());
            m.insert("standard".into(), "IEC 61131-3 LAD subset".into());
            m.insert("features".into(), "or,rto,ctd,move,compare".into());
            m
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_demo() {
        let c = compile(demo_program()).unwrap();
        assert!(!c.instructions.is_empty());
        assert_eq!(c.hash.len(), 64);
        assert!(c
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::OrBegin)));
        assert!(c
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::Compare { .. })));
    }

    #[test]
    fn bytecode_roundtrip() {
        let c = compile(demo_program()).unwrap();
        let bytes = export_bytecode(&c).unwrap();
        let c2 = import_bytecode(&bytes).unwrap();
        assert_eq!(c.hash, c2.hash);
        assert_eq!(c.instructions.len(), c2.instructions.len());
    }

    #[test]
    fn or_empty_branch_errors() {
        let mut p = demo_program();
        p.rungs[0].or_branches.push(vec![]);
        assert!(compile(p).is_err());
    }

    fn single_rung(elements: Vec<LadderElement>) -> LadderProgram {
        LadderProgram {
            name: "t".into(),
            version: "1".into(),
            description: String::new(),
            metadata: BTreeMap::new(),
            rungs: vec![Rung {
                id: "r0".into(),
                comment: String::new(),
                enabled: true,
                or_branches: vec![],
                elements,
            }],
        }
    }

    #[test]
    fn hash_is_deterministic_and_content_sensitive() {
        let h1 = compile(demo_program()).unwrap().hash;
        let h2 = compile(demo_program()).unwrap().hash;
        assert_eq!(h1, h2, "same source ⇒ identical hash");

        let mut changed = demo_program();
        changed.version = "9.9.9".into();
        let h3 = compile(changed).unwrap().hash;
        assert_ne!(h1, h3, "changed program ⇒ different hash");
    }

    #[test]
    fn disabled_rungs_are_skipped() {
        let mut p = single_rung(vec![LadderElement::Coil {
            id: "q".into(),
            address: Address::coil(0),
        }]);
        p.rungs[0].enabled = false;
        let c = compile(p).unwrap();
        assert!(c.instructions.is_empty(), "disabled rung emits no bytecode");
    }

    #[test]
    fn empty_rung_is_rejected() {
        let p = single_rung(vec![]);
        assert!(matches!(compile(p), Err(CompileError::EmptyRung(_))));
    }

    #[test]
    fn holding_bit_addressing_roundtrips() {
        let addr = Address {
            area: MemArea::Holding,
            index: 1,
            bit: Some(3),
        };
        let c = compile(single_rung(vec![
            LadderElement::ContactNo {
                id: "c".into(),
                address: addr,
            },
            LadderElement::Coil {
                id: "q".into(),
                address: Address::coil(0),
            },
        ]))
        .unwrap();
        assert!(c.instructions.iter().any(|i| matches!(
            i,
            Instruction::LoadNo {
                area: MemArea::Holding,
                index: 1,
                bit: Some(3)
            }
        )));

        // Bit survives a bytecode round-trip.
        let c2 = import_bytecode(&export_bytecode(&c).unwrap()).unwrap();
        assert_eq!(c.instructions, c2.instructions);
    }

    // --- Deterministic property/fuzz sweep ---------------------------------
    // The compiler must NEVER panic on any well-typed AST and every successful
    // compile must produce a 64-hex hash and survive a bytecode round-trip.

    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    }

    fn rand_addr(r: &mut u64) -> Address {
        let area = match lcg(r) % 6 {
            0 => MemArea::Coil,
            1 => MemArea::Discrete,
            2 => MemArea::Holding,
            3 => MemArea::InputReg,
            4 => MemArea::MemoryBit,
            _ => MemArea::MemoryWord,
        };
        let index = (lcg(r) % 5000) as u16; // deliberately includes out-of-range
        let bit = if lcg(r) % 2 == 0 {
            Some((lcg(r) % 16) as u8)
        } else {
            None
        };
        Address { area, index, bit }
    }

    fn maybe_addr(r: &mut u64) -> Option<Address> {
        if lcg(r) % 2 == 0 {
            Some(rand_addr(r))
        } else {
            None
        }
    }

    fn rand_element(r: &mut u64) -> LadderElement {
        let id = format!("e{}", lcg(r) % 100_000);
        match lcg(r) % 13 {
            0 => LadderElement::ContactNo {
                id,
                address: rand_addr(r),
            },
            1 => LadderElement::ContactNc {
                id,
                address: rand_addr(r),
            },
            2 => LadderElement::ContactRising {
                id,
                address: rand_addr(r),
            },
            3 => LadderElement::ContactFalling {
                id,
                address: rand_addr(r),
            },
            4 => LadderElement::Coil {
                id,
                address: rand_addr(r),
            },
            5 => LadderElement::CoilSet {
                id,
                address: rand_addr(r),
            },
            6 => LadderElement::Ton {
                id,
                preset_ms: (lcg(r) % 5000) as u32,
                timer_index: (lcg(r) % 60) as u16,
                done_address: maybe_addr(r),
            },
            7 => LadderElement::Ctu {
                id,
                preset: (lcg(r) % 100) as u16,
                counter_index: (lcg(r) % 60) as u16,
                done_address: maybe_addr(r),
                reset_address: maybe_addr(r),
            },
            8 => LadderElement::Math {
                id,
                op: match lcg(r) % 4 {
                    0 => MathOp::Add,
                    1 => MathOp::Sub,
                    2 => MathOp::Mul,
                    _ => MathOp::Div,
                },
                a: rand_addr(r),
                b: rand_addr(r),
                dest: rand_addr(r),
            },
            9 => LadderElement::Move {
                id,
                source: rand_addr(r),
                dest: rand_addr(r),
            },
            10 => LadderElement::Compare {
                id,
                op: CmpOp::Ge,
                a: rand_addr(r),
                b: rand_addr(r),
            },
            11 => LadderElement::Wire { id },
            _ => LadderElement::CoilReset {
                id,
                address: rand_addr(r),
            },
        }
    }

    #[test]
    fn fuzz_random_programs_never_panic_and_roundtrip() {
        let mut r: u64 = 0x9E3779B97F4A7C15;
        for _ in 0..500 {
            let n = 1 + (lcg(&mut r) % 6) as usize;
            let elements: Vec<LadderElement> = (0..n).map(|_| rand_element(&mut r)).collect();
            // Compilation must be total: Ok or a typed Err, never a panic.
            if let Ok(c) = compile(single_rung(elements)) {
                assert_eq!(c.hash.len(), 64);
                let bytes = export_bytecode(&c).unwrap();
                let c2 = import_bytecode(&bytes).unwrap();
                assert_eq!(c.instructions, c2.instructions);
            }
        }
    }

    #[test]
    fn malformed_json_is_rejected_gracefully() {
        for bad in [
            "",
            "{",
            "null",
            "{\"name\":\"x\"}",
            "{\"rungs\": \"not-an-array\"}",
            "[1,2,3]",
        ] {
            let parsed = serde_json::from_str::<LadderProgram>(bad);
            assert!(parsed.is_err(), "malformed input must not parse: {bad:?}");
        }
    }

    #[test]
    fn parallel_group_compiles_to_or_block() {
        let prog = single_rung(vec![
            LadderElement::ContactNo {
                id: "a".into(),
                address: Address::discrete(0),
            },
            LadderElement::ParallelGroup {
                id: "pg".into(),
                branches: vec![
                    vec![LadderElement::ContactNo {
                        id: "b".into(),
                        address: Address::discrete(1),
                    }],
                    vec![LadderElement::ContactNo {
                        id: "c".into(),
                        address: Address::discrete(2),
                    }],
                ],
            },
            LadderElement::Coil {
                id: "q".into(),
                address: Address::coil(0),
            },
        ]);
        let c = compile(prog).unwrap();
        assert!(c
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::OrBegin)));
        assert!(c
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::OrAlt)));
        assert!(c
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::OrEnd)));
        // debug map records the group id for editor highlighting.
        assert!(c.debug_map.contains_key("pg"));
    }

    #[test]
    fn empty_parallel_branch_errors() {
        let prog = single_rung(vec![
            LadderElement::ParallelGroup {
                id: "pg".into(),
                branches: vec![vec![]],
            },
            LadderElement::Coil {
                id: "q".into(),
                address: Address::coil(0),
            },
        ]);
        assert!(compile(prog).is_err());
    }
}
