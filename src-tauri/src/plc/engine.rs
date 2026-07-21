//! =============================================================================
//! PLC Scan Engine — deterministic cyclic execution on a dedicated Tokio task.
//! High thread priority, checked arithmetic, faults → STOP (no panic).
//! Emits live debug events for active element highlighting.
//! =============================================================================

use super::compiler::{CmpOp, CompiledProgram, Instruction, MathOp, MemArea};
use super::memory::{PlcMemory, PlcRunState};
#[cfg(test)]
use super::memory::{COUNTER_HR_BASE, TIMER_HR_BASE};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

/// Event payload for frontend live debugging.
#[derive(Debug, Clone, Serialize)]
pub struct ScanTickEvent {
    pub scan_count: u64,
    pub last_scan_us: u64,
    pub cycle_ms: u32,
    pub run_state: PlcRunState,
    /// Element IDs that had TRUE power rail this scan
    pub active_elements: Vec<String>,
    /// Rung IDs that completed with TRUE power
    pub active_rungs: Vec<String>,
    pub fault_code: u16,
    pub fault_message: String,
}

/// Internal timer instance state (ET in ms, previous IN, Q).
#[derive(Debug, Clone, Default)]
struct TimerState {
    et_ms: u32,
    prev_in: bool,
    q: bool,
}

/// Counter state: CV + edge detect on CU/CD.
#[derive(Debug, Clone, Default)]
struct CounterState {
    cv: u16,
    prev_cu: bool,
    q: bool,
    /// CTD: true after first load so we don't treat 0 as done incorrectly
    loaded: bool,
}

/// Process-image inputs captured once at the beginning of a scan.
#[derive(Debug, Clone)]
struct ScanInputImage {
    discrete_inputs: Vec<bool>,
    input_registers: Vec<u16>,
}

impl ScanInputImage {
    fn capture(memory: &PlcMemory) -> Self {
        let snapshot = memory.snapshot();
        Self {
            discrete_inputs: snapshot.discrete_inputs,
            input_registers: snapshot.input_registers,
        }
    }

    fn discrete(&self, index: u16) -> Result<bool, ScanError> {
        self.discrete_inputs
            .get(index as usize)
            .copied()
            .ok_or_else(|| {
                ScanError::Memory(format!("address out of range in discrete_input: {index}"))
            })
    }

    fn input_reg(&self, index: u16) -> Result<u16, ScanError> {
        self.input_registers
            .get(index as usize)
            .copied()
            .ok_or_else(|| {
                ScanError::Memory(format!("address out of range in input_register: {index}"))
            })
    }
}

/// Shared engine handle used by Tauri commands.
pub struct PlcEngine {
    memory: Arc<PlcMemory>,
    program: RwLock<Option<CompiledProgram>>,
    running: AtomicBool,
    stop_flag: AtomicBool,
    /// Live active element set (last scan)
    last_active: RwLock<HashSet<String>>,
}

impl PlcEngine {
    pub fn new(memory: Arc<PlcMemory>) -> Arc<Self> {
        Arc::new(Self {
            memory,
            program: RwLock::new(None),
            running: AtomicBool::new(false),
            stop_flag: AtomicBool::new(false),
            last_active: RwLock::new(HashSet::new()),
        })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn load_program(&self, program: CompiledProgram) {
        self.memory
            .set_program_meta(program.hash.clone(), program.version.clone());
        *self.program.write() = Some(program);
        info!(
            hash = %self.memory.program_hash(),
            "program loaded into engine"
        );
    }

    pub fn program_snapshot(&self) -> Option<CompiledProgram> {
        self.program.read().clone()
    }

    pub fn last_active_elements(&self) -> Vec<String> {
        self.last_active.read().iter().cloned().collect()
    }

    /// Start scan loop on a high-priority OS thread with its own Tokio runtime.
    pub fn start(self: &Arc<Self>, app: AppHandle) -> Result<(), EngineError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(EngineError::AlreadyRunning);
        }
        self.stop_flag.store(false, Ordering::SeqCst);
        self.memory.clear_fault();
        self.memory.set_run_state(PlcRunState::Run);

        let engine = Arc::clone(self);
        std::thread::Builder::new()
            .name("plc-scan-engine".into())
            .spawn(move || {
                // Elevate priority best-effort (platform dependent)
                if let Err(e) = set_high_priority() {
                    warn!(error = %e, "could not set high thread priority");
                }

                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        error!(error = %e, "failed to build engine runtime");
                        engine.memory.raise_fault(100, format!("runtime: {e}"));
                        engine.running.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                rt.block_on(Arc::clone(&engine).scan_loop(app));
                engine.running.store(false, Ordering::SeqCst);
                if engine.memory.run_state() == PlcRunState::Run {
                    engine.memory.set_run_state(PlcRunState::Stop);
                }
                info!("PLC scan engine stopped");
            })
            .map_err(|e| EngineError::Spawn(e.to_string()))?;

        info!("PLC scan engine started");
        Ok(())
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if self.memory.run_state() == PlcRunState::Run {
            self.memory.set_run_state(PlcRunState::Stop);
        }
    }

    async fn scan_loop(self: Arc<Self>, app: AppHandle) {
        let mut timers: HashMap<u16, TimerState> = HashMap::new();
        let mut counters: HashMap<u16, CounterState> = HashMap::new();
        // Previous bit level per edge-contact element id (rising/falling)
        let mut edges: HashMap<String, bool> = HashMap::new();

        loop {
            if self.stop_flag.load(Ordering::SeqCst) {
                break;
            }
            if self.memory.run_state() == PlcRunState::Fault {
                break;
            }

            let cycle_ms = self.memory.cycle_ms().clamp(5, 100);
            let period = Duration::from_millis(cycle_ms as u64);
            let tick_start = Instant::now();

            let result = self.execute_scan(cycle_ms, &mut timers, &mut counters, &mut edges);

            match result {
                Ok(tick) => {
                    let _ = app.emit("plc://scan-tick", &tick);
                    // Canonical UI compact image (covers R40–R42 demo, I/Q 0–127, M/MR head)
                    let snap = self.memory.snapshot_ui();
                    let _ = app.emit("plc://memory", &snap);
                }
                Err(e) => {
                    error!(error = %e, "scan fault — entering STOP");
                    self.memory.raise_fault(e.code(), e.to_string());
                    let tick = ScanTickEvent {
                        scan_count: self.memory.scan_count(),
                        last_scan_us: self.memory.last_scan_us(),
                        cycle_ms,
                        run_state: PlcRunState::Fault,
                        active_elements: Vec::new(),
                        active_rungs: Vec::new(),
                        fault_code: e.code(),
                        fault_message: e.to_string(),
                    };
                    let _ = app.emit("plc://scan-tick", &tick);
                    let _ = app.emit("plc://fault", &tick);
                    break;
                }
            }

            let elapsed = tick_start.elapsed();
            if elapsed < period {
                tokio::time::sleep(period - elapsed).await;
            } else {
                // Overrun: continue immediately; diagnostic only
                warn!(
                    elapsed_us = elapsed.as_micros(),
                    cycle_ms, "scan cycle overrun"
                );
            }
        }
    }

    fn execute_scan(
        &self,
        cycle_ms: u32,
        timers: &mut HashMap<u16, TimerState>,
        counters: &mut HashMap<u16, CounterState>,
        edges: &mut HashMap<String, bool>,
    ) -> Result<ScanTickEvent, ScanError> {
        let start = Instant::now();
        let program = self.program.read();
        let Some(prog) = program.as_ref() else {
            // No program: idle scan still advances counter
            let us = start.elapsed().as_micros() as u64;
            self.memory.increment_scan(us);
            return Ok(ScanTickEvent {
                scan_count: self.memory.scan_count(),
                last_scan_us: us,
                cycle_ms,
                run_state: self.memory.run_state(),
                active_elements: Vec::new(),
                active_rungs: Vec::new(),
                fault_code: 0,
                fault_message: String::new(),
            });
        };

        let mut power = true; // each rung starts with left power rail TRUE
        let mut or_accum = false;
        // Stack of (incoming_power, outer_or_accum) so a parallel group placed
        // mid-rung is AND-ed with the series power that reached it (and nesting works).
        let mut or_stack: Vec<(bool, bool)> = Vec::new();
        let mut active_elements: HashSet<String> = HashSet::new();
        let mut active_rungs: HashSet<String> = HashSet::new();
        let input_image = ScanInputImage::capture(&self.memory);
        let inv_debug: HashMap<usize, String> = prog
            .debug_map
            .iter()
            .map(|(k, v)| (*v, k.clone()))
            .collect();

        for (i, instr) in prog.instructions.iter().enumerate() {
            match instr {
                Instruction::Nop => {
                    if let Some(id) = inv_debug.get(&i) {
                        if power {
                            active_elements.insert(id.clone());
                        }
                    }
                }
                Instruction::OrBegin => {
                    or_stack.push((power, or_accum));
                    or_accum = false;
                    power = true;
                }
                Instruction::OrAlt => {
                    or_accum |= power;
                    power = true;
                }
                Instruction::OrEnd => {
                    or_accum |= power;
                    let (incoming, outer_accum) = or_stack.pop().unwrap_or((true, false));
                    power = incoming && or_accum;
                    or_accum = outer_accum;
                }
                Instruction::LoadNo { area, index, bit } => {
                    let v = self.read_bit(*area, *index, *bit, &input_image)?;
                    power = power && v;
                    if let Some(id) = inv_debug.get(&i) {
                        if power {
                            active_elements.insert(id.clone());
                        }
                    }
                }
                Instruction::LoadNc { area, index, bit } => {
                    let v = self.read_bit(*area, *index, *bit, &input_image)?;
                    power = power && !v;
                    if let Some(id) = inv_debug.get(&i) {
                        if power {
                            active_elements.insert(id.clone());
                        }
                    }
                }
                Instruction::LoadRising {
                    area,
                    index,
                    bit,
                    element_id,
                } => {
                    let now = self.read_bit(*area, *index, *bit, &input_image)?;
                    let prev = edges.get(element_id).copied().unwrap_or(false);
                    let edge = !prev && now; // 0 → 1
                    edges.insert(element_id.clone(), now);
                    power = power && edge;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::LoadFalling {
                    area,
                    index,
                    bit,
                    element_id,
                } => {
                    let now = self.read_bit(*area, *index, *bit, &input_image)?;
                    let prev = edges.get(element_id).copied().unwrap_or(false);
                    let edge = prev && !now; // 1 → 0
                    edges.insert(element_id.clone(), now);
                    power = power && edge;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::StoreCoil { area, index, bit } => {
                    self.write_bit(*area, *index, *bit, power)?;
                    if let Some(id) = inv_debug.get(&i) {
                        if power {
                            active_elements.insert(id.clone());
                        }
                    }
                }
                Instruction::StoreCoilNegated { area, index, bit } => {
                    self.write_bit(*area, *index, *bit, !power)?;
                    if let Some(id) = inv_debug.get(&i) {
                        if power {
                            active_elements.insert(id.clone());
                        }
                    }
                }
                Instruction::StoreSet { area, index, bit } => {
                    // SET (latch): force 1 while powered; hold previous value when power drops
                    if power {
                        self.write_bit(*area, *index, *bit, true)?;
                    }
                    // Highlight whenever the bit is latched OR power is present this scan
                    let latched = self.read_bit_live(*area, *index, *bit)?;
                    if let Some(id) = inv_debug.get(&i) {
                        if power || latched {
                            active_elements.insert(id.clone());
                        }
                    }
                    // Power rail continues with current power (does not follow latched bit)
                }
                Instruction::StoreReset { area, index, bit } => {
                    // RESET (unlatch): force 0 while powered
                    if power {
                        self.write_bit(*area, *index, *bit, false)?;
                        if let Some(id) = inv_debug.get(&i) {
                            active_elements.insert(id.clone());
                        }
                    }
                }
                Instruction::Ton {
                    preset_ms,
                    timer_index,
                    done,
                    element_id,
                } => {
                    let st = timers.entry(*timer_index).or_default();
                    if power {
                        st.et_ms = st.et_ms.saturating_add(cycle_ms);
                        if st.et_ms >= *preset_ms {
                            st.et_ms = *preset_ms;
                            st.q = true;
                        }
                    } else {
                        st.et_ms = 0;
                        st.q = false;
                    }
                    st.prev_in = power;
                    self.write_timer_regs(*timer_index, st)?;
                    if let Some((a, idx, bit)) = done {
                        self.write_bit(*a, *idx, *bit, st.q)?;
                    }
                    power = st.q;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::Tof {
                    preset_ms,
                    timer_index,
                    done,
                    element_id,
                } => {
                    let st = timers.entry(*timer_index).or_default();
                    if power {
                        st.et_ms = 0;
                        st.q = true;
                    } else if st.prev_in || st.q {
                        st.et_ms = st.et_ms.saturating_add(cycle_ms);
                        if st.et_ms >= *preset_ms {
                            st.et_ms = *preset_ms;
                            st.q = false;
                        } else {
                            st.q = true;
                        }
                    }
                    st.prev_in = power;
                    self.write_timer_regs(*timer_index, st)?;
                    if let Some((a, idx, bit)) = done {
                        self.write_bit(*a, *idx, *bit, st.q)?;
                    }
                    power = st.q;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::Rto {
                    preset_ms,
                    timer_index,
                    done,
                    reset,
                    element_id,
                } => {
                    let st = timers.entry(*timer_index).or_default();
                    let rst = if let Some((a, idx, bit)) = reset {
                        self.read_bit(*a, *idx, *bit, &input_image)?
                    } else {
                        false
                    };
                    if rst {
                        st.et_ms = 0;
                        st.q = false;
                    } else if power {
                        // retentive: accumulate only while IN; hold ET when IN false
                        st.et_ms = st.et_ms.saturating_add(cycle_ms);
                        if st.et_ms >= *preset_ms {
                            st.et_ms = *preset_ms;
                            st.q = true;
                        }
                    }
                    st.prev_in = power;
                    self.write_timer_regs(*timer_index, st)?;
                    if let Some((a, idx, bit)) = done {
                        self.write_bit(*a, *idx, *bit, st.q)?;
                    }
                    power = st.q;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::Ctu {
                    preset,
                    counter_index,
                    done,
                    reset,
                    element_id,
                } => {
                    let st = counters.entry(*counter_index).or_default();
                    let reset_active = if let Some((a, idx, bit)) = reset {
                        self.read_bit(*a, *idx, *bit, &input_image)?
                    } else {
                        false
                    };
                    if reset_active {
                        st.cv = 0;
                        st.q = false;
                    } else if power && !st.prev_cu {
                        st.cv = st.cv.saturating_add(1);
                        if st.cv >= *preset {
                            st.q = true;
                        }
                    }
                    st.prev_cu = power;
                    self.write_counter_regs(*counter_index, st)?;
                    if let Some((a, idx, bit)) = done {
                        self.write_bit(*a, *idx, *bit, st.q)?;
                    }
                    power = st.q;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::Ctd {
                    preset,
                    counter_index,
                    done,
                    load,
                    element_id,
                } => {
                    let st = counters.entry(*counter_index).or_default();
                    let load_active = if let Some((a, idx, bit)) = load {
                        self.read_bit(*a, *idx, *bit, &input_image)?
                    } else {
                        false
                    };
                    if load_active {
                        st.cv = *preset;
                        st.loaded = true;
                        st.q = false;
                    } else if power && !st.prev_cu && st.loaded {
                        st.cv = st.cv.saturating_sub(1);
                        if st.cv == 0 {
                            st.q = true;
                        }
                    }
                    st.prev_cu = power;
                    self.write_counter_regs(*counter_index, st)?;
                    if let Some((a, idx, bit)) = done {
                        self.write_bit(*a, *idx, *bit, st.q)?;
                    }
                    power = st.q;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::Math {
                    op,
                    a,
                    b,
                    dest,
                    element_id,
                } => {
                    if power {
                        let va = self.read_word(a.0, a.1, &input_image)?;
                        let vb = self.read_word(b.0, b.1, &input_image)?;
                        let result = match op {
                            MathOp::Add => {
                                va.checked_add(vb).ok_or(ScanError::ArithmeticOverflow)?
                            }
                            MathOp::Sub => {
                                va.checked_sub(vb).ok_or(ScanError::ArithmeticOverflow)?
                            }
                            MathOp::Mul => {
                                va.checked_mul(vb).ok_or(ScanError::ArithmeticOverflow)?
                            }
                            MathOp::Div => {
                                if vb == 0 {
                                    return Err(ScanError::DivideByZero);
                                }
                                va / vb
                            }
                        };
                        self.write_word(dest.0, dest.1, result)?;
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::Move {
                    source,
                    dest,
                    element_id,
                } => {
                    if power {
                        let v = self.read_word(source.0, source.1, &input_image)?;
                        self.write_word(dest.0, dest.1, v)?;
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::Compare {
                    op,
                    a,
                    b,
                    element_id,
                } => {
                    let va = self.read_word(a.0, a.1, &input_image)?;
                    let vb = self.read_word(b.0, b.1, &input_image)?;
                    let ok = match op {
                        CmpOp::Eq => va == vb,
                        CmpOp::Ne => va != vb,
                        CmpOp::Gt => va > vb,
                        CmpOp::Ge => va >= vb,
                        CmpOp::Lt => va < vb,
                        CmpOp::Le => va <= vb,
                    };
                    power = power && ok;
                    if power {
                        active_elements.insert(element_id.clone());
                    }
                }
                Instruction::EndRung { rung_id } => {
                    if power {
                        active_rungs.insert(rung_id.clone());
                    }
                    power = true;
                    or_accum = false;
                    or_stack.clear();
                }
            }
        }

        let us = start.elapsed().as_micros() as u64;
        self.memory.increment_scan(us);
        *self.last_active.write() = active_elements.clone();

        Ok(ScanTickEvent {
            scan_count: self.memory.scan_count(),
            last_scan_us: us,
            cycle_ms,
            run_state: self.memory.run_state(),
            active_elements: active_elements.into_iter().collect(),
            active_rungs: active_rungs.into_iter().collect(),
            fault_code: 0,
            fault_message: String::new(),
        })
    }

    fn write_timer_regs(&self, timer_index: u16, st: &TimerState) -> Result<(), ScanError> {
        // UI image + holding bank R2048+ (TV / T done bit for MOVE/contacts).
        self.memory
            .set_timer_image(timer_index, st.et_ms.min(u16::MAX as u32) as u16, st.q)
            .map_err(|e| ScanError::Memory(e.to_string()))
    }

    fn write_counter_regs(&self, counter_index: u16, st: &CounterState) -> Result<(), ScanError> {
        // UI image + holding bank R3072+ (CV / C done bit for MOVE/contacts).
        self.memory
            .set_counter_image(counter_index, st.cv, st.q)
            .map_err(|e| ScanError::Memory(e.to_string()))
    }

    fn read_bit(
        &self,
        area: MemArea,
        index: u16,
        bit: Option<u8>,
        input_image: &ScanInputImage,
    ) -> Result<bool, ScanError> {
        match area {
            MemArea::Discrete => input_image.discrete(index),
            MemArea::InputReg => {
                let v = input_image.input_reg(index)?;
                Ok(match bit {
                    Some(b) => (v >> (b.min(15))) & 1 == 1,
                    None => v != 0,
                })
            }
            MemArea::Coil | MemArea::Holding | MemArea::MemoryBit | MemArea::MemoryWord => {
                self.read_bit_live(area, index, bit)
            }
        }
    }

    fn read_bit_live(&self, area: MemArea, index: u16, bit: Option<u8>) -> Result<bool, ScanError> {
        match area {
            MemArea::Coil => self
                .memory
                .get_coil(index)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::Discrete => self
                .memory
                .get_discrete(index)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::Holding => {
                let v = self
                    .memory
                    .get_holding(index)
                    .map_err(|e| ScanError::Memory(e.to_string()))?;
                Ok(match bit {
                    Some(b) => (v >> (b.min(15))) & 1 == 1,
                    None => v != 0,
                })
            }
            MemArea::InputReg => {
                let v = self
                    .memory
                    .get_input_reg(index)
                    .map_err(|e| ScanError::Memory(e.to_string()))?;
                Ok(match bit {
                    Some(b) => (v >> (b.min(15))) & 1 == 1,
                    None => v != 0,
                })
            }
            MemArea::MemoryBit => self
                .memory
                .get_memory_bit(index)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::MemoryWord => {
                let v = self
                    .memory
                    .get_memory_word(index)
                    .map_err(|e| ScanError::Memory(e.to_string()))?;
                Ok(match bit {
                    Some(b) => (v >> (b.min(15))) & 1 == 1,
                    None => v != 0,
                })
            }
        }
    }

    fn write_bit(
        &self,
        area: MemArea,
        index: u16,
        bit: Option<u8>,
        value: bool,
    ) -> Result<(), ScanError> {
        match area {
            MemArea::Coil => self
                .memory
                .set_coil(index, value)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::Discrete => self
                .memory
                .set_discrete(index, value)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::Holding => {
                if let Some(b) = bit {
                    let b = b.min(15);
                    let cur = self
                        .memory
                        .get_holding(index)
                        .map_err(|e| ScanError::Memory(e.to_string()))?;
                    let mask = 1u16 << b;
                    let next = if value { cur | mask } else { cur & !mask };
                    self.memory
                        .set_holding(index, next)
                        .map_err(|e| ScanError::Memory(e.to_string()))
                } else {
                    self.memory
                        .set_holding(index, if value { 1 } else { 0 })
                        .map_err(|e| ScanError::Memory(e.to_string()))
                }
            }
            MemArea::InputReg => {
                if let Some(b) = bit {
                    let b = b.min(15);
                    let cur = self
                        .memory
                        .get_input_reg(index)
                        .map_err(|e| ScanError::Memory(e.to_string()))?;
                    let mask = 1u16 << b;
                    let next = if value { cur | mask } else { cur & !mask };
                    self.memory
                        .set_input_reg(index, next)
                        .map_err(|e| ScanError::Memory(e.to_string()))
                } else {
                    self.memory
                        .set_input_reg(index, if value { 1 } else { 0 })
                        .map_err(|e| ScanError::Memory(e.to_string()))
                }
            }
            MemArea::MemoryBit => self
                .memory
                .set_memory_bit(index, value)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::MemoryWord => {
                if let Some(b) = bit {
                    let b = b.min(15);
                    let cur = self
                        .memory
                        .get_memory_word(index)
                        .map_err(|e| ScanError::Memory(e.to_string()))?;
                    let mask = 1u16 << b;
                    let next = if value { cur | mask } else { cur & !mask };
                    self.memory
                        .set_memory_word(index, next)
                        .map_err(|e| ScanError::Memory(e.to_string()))
                } else {
                    self.memory
                        .set_memory_word(index, if value { 1 } else { 0 })
                        .map_err(|e| ScanError::Memory(e.to_string()))
                }
            }
        }
    }

    fn read_word(
        &self,
        area: MemArea,
        index: u16,
        input_image: &ScanInputImage,
    ) -> Result<u16, ScanError> {
        match area {
            MemArea::Holding => self
                .memory
                .get_holding(index)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::InputReg => input_image.input_reg(index),
            MemArea::Coil => {
                let b = self
                    .memory
                    .get_coil(index)
                    .map_err(|e| ScanError::Memory(e.to_string()))?;
                Ok(if b { 1 } else { 0 })
            }
            MemArea::Discrete => {
                let b = input_image.discrete(index)?;
                Ok(if b { 1 } else { 0 })
            }
            MemArea::MemoryWord => self
                .memory
                .get_memory_word(index)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::MemoryBit => {
                let b = self
                    .memory
                    .get_memory_bit(index)
                    .map_err(|e| ScanError::Memory(e.to_string()))?;
                Ok(if b { 1 } else { 0 })
            }
        }
    }

    fn write_word(&self, area: MemArea, index: u16, value: u16) -> Result<(), ScanError> {
        match area {
            MemArea::Holding => self
                .memory
                .set_holding(index, value)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::InputReg => self
                .memory
                .set_input_reg(index, value)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::Coil => self
                .memory
                .set_coil(index, value != 0)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::Discrete => self
                .memory
                .set_discrete(index, value != 0)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::MemoryWord => self
                .memory
                .set_memory_word(index, value)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::MemoryBit => self
                .memory
                .set_memory_bit(index, value != 0)
                .map_err(|e| ScanError::Memory(e.to_string())),
        }
    }
}

fn set_high_priority() -> Result<(), String> {
    use thread_priority::{set_current_thread_priority, ThreadPriority};
    set_current_thread_priority(ThreadPriority::Max).map_err(|e| format!("{e:?}"))
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("engine already running")]
    AlreadyRunning,
    #[error("failed to spawn engine thread: {0}")]
    Spawn(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("memory error: {0}")]
    Memory(String),
    #[error("arithmetic overflow")]
    ArithmeticOverflow,
    #[error("divide by zero")]
    DivideByZero,
}

impl ScanError {
    pub fn code(&self) -> u16 {
        match self {
            ScanError::Memory(_) => 1,
            ScanError::ArithmeticOverflow => 2,
            ScanError::DivideByZero => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plc::compiler::{
        compile, demo_program, Address, LadderElement, LadderProgram, Rung,
    };
    use std::collections::BTreeMap;

    fn run_n(engine: &PlcEngine, n: u32, cycle_ms: u32) {
        let mut timers = HashMap::new();
        let mut counters = HashMap::new();
        let mut edges = HashMap::new();
        for _ in 0..n {
            engine
                .execute_scan(cycle_ms, &mut timers, &mut counters, &mut edges)
                .expect("scan");
        }
    }

    fn single_rung_program(name: &str, elements: Vec<LadderElement>) -> LadderProgram {
        LadderProgram {
            name: name.into(),
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

    #[derive(Default)]
    struct ScanState {
        timers: HashMap<u16, TimerState>,
        counters: HashMap<u16, CounterState>,
        edges: HashMap<String, bool>,
    }

    impl ScanState {
        fn scan(&mut self, engine: &PlcEngine, cycle_ms: u32) {
            engine
                .execute_scan(
                    cycle_ms,
                    &mut self.timers,
                    &mut self.counters,
                    &mut self.edges,
                )
                .expect("scan");
        }

        fn scan_n(&mut self, engine: &PlcEngine, n: u32, cycle_ms: u32) {
            for _ in 0..n {
                self.scan(engine, cycle_ms);
            }
        }
    }

    #[test]
    fn or_seal_in_holds_q0() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        eng.load_program(compile(demo_program()).unwrap());

        mem.set_discrete(0, true).unwrap(); // start
        mem.set_discrete(1, false).unwrap();
        run_n(&eng, 2, 20);
        assert!(mem.get_coil(0).unwrap(), "Q0 should set on start");

        mem.set_discrete(0, false).unwrap(); // release start — seal via OR Q0
        run_n(&eng, 2, 20);
        assert!(mem.get_coil(0).unwrap(), "Q0 should seal-in");

        mem.set_discrete(1, true).unwrap(); // stop
        run_n(&eng, 2, 20);
        assert!(!mem.get_coil(0).unwrap(), "Q0 should drop on stop");
    }

    #[test]
    fn internal_memory_bit_drives_coil() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "mbit",
            vec![
                LadderElement::ContactNo {
                    id: "c0".into(),
                    address: Address {
                        area: MemArea::MemoryBit,
                        index: 5,
                        bit: None,
                    },
                },
                LadderElement::Coil {
                    id: "q0".into(),
                    address: Address::coil(0),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());

        mem.set_memory_bit(5, true).unwrap();
        run_n(&eng, 1, 20);
        assert!(mem.get_coil(0).unwrap(), "Q0 follows internal marker M5");

        mem.set_memory_bit(5, false).unwrap();
        run_n(&eng, 1, 20);
        assert!(!mem.get_coil(0).unwrap(), "Q0 clears when M5 clears");
    }

    #[test]
    fn internal_memory_register_moves_both_ways() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));

        // MR0 → R7 (read internal register as source)
        let prog = single_rung_program(
            "mr_read",
            vec![LadderElement::Move {
                id: "m".into(),
                source: Address {
                    area: MemArea::MemoryWord,
                    index: 0,
                    bit: None,
                },
                dest: Address::holding(7),
            }],
        );
        eng.load_program(compile(prog).unwrap());
        mem.set_memory_word(0, 4321).unwrap();
        run_n(&eng, 1, 20);
        assert_eq!(mem.get_holding(7).unwrap(), 4321, "MR0 usable as source");

        // R8 → MR3 (write internal register as destination)
        let prog2 = single_rung_program(
            "mr_write",
            vec![LadderElement::Move {
                id: "m2".into(),
                source: Address::holding(8),
                dest: Address {
                    area: MemArea::MemoryWord,
                    index: 3,
                    bit: None,
                },
            }],
        );
        eng.load_program(compile(prog2).unwrap());
        mem.set_holding(8, 777).unwrap();
        run_n(&eng, 1, 20);
        assert_eq!(mem.get_memory_word(3).unwrap(), 777, "MR3 usable as dest");
    }

    #[test]
    fn compare_and_move() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        eng.load_program(compile(demo_program()).unwrap());
        mem.set_holding(40, 100).unwrap();
        mem.set_holding(41, 50).unwrap();
        run_n(&eng, 1, 20);
        assert_eq!(mem.get_holding(42).unwrap(), 100);
        assert!(mem.get_coil(3).unwrap());
    }

    #[test]
    fn divide_by_zero_faults() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = LadderProgram {
            name: "div0".into(),
            version: "1".into(),
            description: String::new(),
            metadata: BTreeMap::new(),
            rungs: vec![Rung {
                id: "r0".into(),
                comment: String::new(),
                enabled: true,
                or_branches: vec![],
                elements: vec![LadderElement::Math {
                    id: "m".into(),
                    op: crate::plc::compiler::MathOp::Div,
                    a: Address::holding(0),
                    b: Address::holding(1),
                    dest: Address::holding(2),
                }],
            }],
        };
        eng.load_program(compile(prog).unwrap());
        mem.set_holding(0, 10).unwrap();
        mem.set_holding(1, 0).unwrap();
        let mut timers = HashMap::new();
        let mut counters = HashMap::new();
        let mut edges = HashMap::new();
        let err = eng.execute_scan(20, &mut timers, &mut counters, &mut edges);
        assert!(err.is_err());
    }

    #[test]
    fn rto_retains_et() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = LadderProgram {
            name: "rto".into(),
            version: "1".into(),
            description: String::new(),
            metadata: BTreeMap::new(),
            rungs: vec![Rung {
                id: "r0".into(),
                comment: String::new(),
                enabled: true,
                or_branches: vec![],
                elements: vec![
                    LadderElement::ContactNo {
                        id: "c".into(),
                        address: Address::discrete(0),
                    },
                    LadderElement::Rto {
                        id: "rto".into(),
                        preset_ms: 100,
                        timer_index: 5,
                        done_address: Some(Address::coil(7)),
                        reset_address: Some(Address::discrete(1)),
                    },
                ],
            }],
        };
        eng.load_program(compile(prog).unwrap());
        let mut timers = HashMap::new();
        let mut counters = HashMap::new();
        let mut edges = HashMap::new();
        mem.set_discrete(0, true).unwrap();
        for _ in 0..3 {
            eng.execute_scan(20, &mut timers, &mut counters, &mut edges)
                .unwrap();
        }
        let et_addr = TIMER_HR_BASE + 5 * 2;
        let et_mid = mem.get_holding(et_addr).unwrap();
        assert!(et_mid >= 60);
        mem.set_discrete(0, false).unwrap();
        eng.execute_scan(20, &mut timers, &mut counters, &mut edges)
            .unwrap();
        let et_hold = mem.get_holding(et_addr).unwrap();
        assert_eq!(et_hold, et_mid, "RTO must retain ET when IN drops");
    }

    #[test]
    fn rising_edge_contact_fires_for_one_scan() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "edge",
            vec![
                LadderElement::ContactRising {
                    id: "edge".into(),
                    address: Address::discrete(0),
                },
                LadderElement::Coil {
                    id: "q".into(),
                    address: Address::coil(0),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());

        let mut timers = HashMap::new();
        let mut counters = HashMap::new();
        let mut edges = HashMap::new();

        mem.set_discrete(0, false).unwrap();
        eng.execute_scan(20, &mut timers, &mut counters, &mut edges)
            .unwrap();
        assert!(!mem.get_coil(0).unwrap());

        mem.set_discrete(0, true).unwrap();
        eng.execute_scan(20, &mut timers, &mut counters, &mut edges)
            .unwrap();
        assert!(mem.get_coil(0).unwrap(), "rising edge should pulse once");

        eng.execute_scan(20, &mut timers, &mut counters, &mut edges)
            .unwrap();
        assert!(!mem.get_coil(0).unwrap(), "held input must not retrigger");
    }

    #[test]
    fn set_reset_latch_holds_until_reset() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = LadderProgram {
            name: "set_reset".into(),
            version: "1".into(),
            description: String::new(),
            metadata: BTreeMap::new(),
            rungs: vec![
                Rung {
                    id: "set".into(),
                    comment: String::new(),
                    enabled: true,
                    or_branches: vec![],
                    elements: vec![
                        LadderElement::ContactNo {
                            id: "set_in".into(),
                            address: Address::discrete(0),
                        },
                        LadderElement::CoilSet {
                            id: "set_q".into(),
                            address: Address::coil(0),
                        },
                    ],
                },
                Rung {
                    id: "reset".into(),
                    comment: String::new(),
                    enabled: true,
                    or_branches: vec![],
                    elements: vec![
                        LadderElement::ContactNo {
                            id: "reset_in".into(),
                            address: Address::discrete(1),
                        },
                        LadderElement::CoilReset {
                            id: "reset_q".into(),
                            address: Address::coil(0),
                        },
                    ],
                },
            ],
        };
        eng.load_program(compile(prog).unwrap());

        mem.set_discrete(0, true).unwrap();
        run_n(&eng, 1, 20);
        assert!(mem.get_coil(0).unwrap());

        mem.set_discrete(0, false).unwrap();
        run_n(&eng, 1, 20);
        assert!(mem.get_coil(0).unwrap(), "SET coil should retain state");

        mem.set_discrete(1, true).unwrap();
        run_n(&eng, 1, 20);
        assert!(!mem.get_coil(0).unwrap(), "RESET coil should clear latch");
    }

    #[test]
    fn ton_waits_until_preset() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "ton",
            vec![
                LadderElement::ContactNo {
                    id: "in".into(),
                    address: Address::discrete(0),
                },
                LadderElement::Ton {
                    id: "ton".into(),
                    preset_ms: 60,
                    timer_index: 0,
                    done_address: Some(Address::coil(1)),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        mem.set_discrete(0, true).unwrap();
        let mut scan = ScanState::default();

        scan.scan_n(&eng, 2, 20);
        assert!(!mem.get_coil(1).unwrap());
        scan.scan(&eng, 20);
        assert!(mem.get_coil(1).unwrap());
    }

    #[test]
    fn tof_delays_drop_after_input_falls() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "tof",
            vec![
                LadderElement::ContactNo {
                    id: "in".into(),
                    address: Address::discrete(0),
                },
                LadderElement::Tof {
                    id: "tof".into(),
                    preset_ms: 60,
                    timer_index: 0,
                    done_address: Some(Address::coil(1)),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        let mut scan = ScanState::default();

        mem.set_discrete(0, true).unwrap();
        scan.scan(&eng, 20);
        assert!(mem.get_coil(1).unwrap());

        mem.set_discrete(0, false).unwrap();
        scan.scan_n(&eng, 2, 20);
        assert!(mem.get_coil(1).unwrap());
        scan.scan(&eng, 20);
        assert!(!mem.get_coil(1).unwrap());
    }

    #[test]
    fn ctu_counts_rising_edges_and_reset_clears() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "ctu",
            vec![
                LadderElement::ContactNo {
                    id: "pulse".into(),
                    address: Address::discrete(0),
                },
                LadderElement::Ctu {
                    id: "ctu".into(),
                    preset: 3,
                    counter_index: 0,
                    done_address: Some(Address::coil(2)),
                    reset_address: Some(Address::discrete(1)),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        let mut scan = ScanState::default();

        for _ in 0..3 {
            mem.set_discrete(0, true).unwrap();
            scan.scan(&eng, 20);
            mem.set_discrete(0, false).unwrap();
            scan.scan(&eng, 20);
        }
        assert!(mem.get_coil(2).unwrap());
        assert_eq!(mem.get_holding(COUNTER_HR_BASE).unwrap(), 3);

        mem.set_discrete(1, true).unwrap();
        scan.scan(&eng, 20);
        assert!(!mem.get_coil(2).unwrap());
        assert_eq!(mem.get_holding(COUNTER_HR_BASE).unwrap(), 0);
    }

    #[test]
    fn ctd_loads_and_counts_down_to_done() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "ctd",
            vec![
                LadderElement::ContactNo {
                    id: "pulse".into(),
                    address: Address::discrete(0),
                },
                LadderElement::Ctd {
                    id: "ctd".into(),
                    preset: 2,
                    counter_index: 2,
                    done_address: Some(Address::coil(3)),
                    load_address: Some(Address::discrete(1)),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        let mut scan = ScanState::default();

        mem.set_discrete(1, true).unwrap();
        scan.scan(&eng, 20);
        mem.set_discrete(1, false).unwrap();
        let c2 = COUNTER_HR_BASE + 2 * 2;
        assert_eq!(mem.get_holding(c2).unwrap(), 2);

        for _ in 0..2 {
            mem.set_discrete(0, true).unwrap();
            scan.scan(&eng, 20);
            mem.set_discrete(0, false).unwrap();
            scan.scan(&eng, 20);
        }
        assert!(mem.get_coil(3).unwrap());
        assert_eq!(mem.get_holding(c2).unwrap(), 0);
    }

    #[test]
    fn discrete_inputs_are_snapshotted_for_current_scan() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = LadderProgram {
            name: "input_snapshot".into(),
            version: "1".into(),
            description: String::new(),
            metadata: BTreeMap::new(),
            rungs: vec![
                Rung {
                    id: "write_input".into(),
                    comment: String::new(),
                    enabled: true,
                    or_branches: vec![],
                    elements: vec![
                        LadderElement::ContactNo {
                            id: "trigger".into(),
                            address: Address::discrete(0),
                        },
                        LadderElement::Coil {
                            id: "write_i1".into(),
                            address: Address::discrete(1),
                        },
                    ],
                },
                Rung {
                    id: "read_input".into(),
                    comment: String::new(),
                    enabled: true,
                    or_branches: vec![],
                    elements: vec![
                        LadderElement::ContactNo {
                            id: "read_i1".into(),
                            address: Address::discrete(1),
                        },
                        LadderElement::Coil {
                            id: "q".into(),
                            address: Address::coil(0),
                        },
                    ],
                },
            ],
        };
        eng.load_program(compile(prog).unwrap());

        mem.set_discrete(0, true).unwrap();
        run_n(&eng, 1, 20);
        assert!(mem.get_discrete(1).unwrap());
        assert!(
            !mem.get_coil(0).unwrap(),
            "same-scan reads must use the frozen input image"
        );

        run_n(&eng, 1, 20);
        assert!(mem.get_coil(0).unwrap());
    }

    #[test]
    fn math_overflow_faults() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "ovf",
            vec![LadderElement::Math {
                id: "m".into(),
                op: crate::plc::compiler::MathOp::Add,
                a: Address::holding(0),
                b: Address::holding(1),
                dest: Address::holding(2),
            }],
        );
        eng.load_program(compile(prog).unwrap());
        mem.set_holding(0, u16::MAX).unwrap();
        mem.set_holding(1, 1).unwrap();
        let mut t = HashMap::new();
        let mut c = HashMap::new();
        let mut e = HashMap::new();
        assert!(
            eng.execute_scan(20, &mut t, &mut c, &mut e).is_err(),
            "u16 overflow must fault, not wrap or panic"
        );
    }

    #[test]
    fn holding_bit_write_preserves_other_bits() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let addr = Address {
            area: crate::plc::compiler::MemArea::Holding,
            index: 5,
            bit: Some(3),
        };
        let prog = single_rung_program(
            "bit",
            vec![
                LadderElement::ContactNo {
                    id: "c".into(),
                    address: Address::discrete(0),
                },
                LadderElement::Coil {
                    id: "q".into(),
                    address: addr,
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        mem.set_holding(5, 0b0000_0001).unwrap(); // preset bit 0
        mem.set_discrete(0, true).unwrap();
        run_n(&eng, 1, 20);
        let v = mem.get_holding(5).unwrap();
        assert_eq!(v & (1 << 3), 1 << 3, "bit 3 set by masked write");
        assert_eq!(v & 1, 1, "bit 0 must be preserved");
    }

    #[test]
    fn falling_edge_contact_fires_one_scan() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "fall",
            vec![
                LadderElement::ContactFalling {
                    id: "f".into(),
                    address: Address::discrete(0),
                },
                LadderElement::Coil {
                    id: "q".into(),
                    address: Address::coil(0),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        let mut s = ScanState::default();
        mem.set_discrete(0, true).unwrap();
        s.scan(&eng, 20); // establish high level
        assert!(!mem.get_coil(0).unwrap());
        mem.set_discrete(0, false).unwrap();
        s.scan(&eng, 20); // 1→0 transition fires
        assert!(mem.get_coil(0).unwrap());
        s.scan(&eng, 20); // one-shot: no longer firing
        assert!(!mem.get_coil(0).unwrap());
    }

    #[test]
    fn coil_negated_writes_inverse_of_power() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        let prog = single_rung_program(
            "neg",
            vec![
                LadderElement::ContactNo {
                    id: "c".into(),
                    address: Address::discrete(0),
                },
                LadderElement::CoilNegated {
                    id: "q".into(),
                    address: Address::coil(0),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        run_n(&eng, 1, 20); // power false ⇒ negated coil TRUE
        assert!(mem.get_coil(0).unwrap());
        mem.set_discrete(0, true).unwrap();
        run_n(&eng, 1, 20); // power true ⇒ negated coil FALSE
        assert!(!mem.get_coil(0).unwrap());
    }

    #[test]
    fn inline_parallel_group_ands_with_incoming_power() {
        let mem = PlcMemory::new().into_arc();
        let eng = PlcEngine::new(Arc::clone(&mem));
        // I0 AND (I1 OR I2) → Q0
        let prog = single_rung_program(
            "midpar",
            vec![
                LadderElement::ContactNo {
                    id: "i0".into(),
                    address: Address::discrete(0),
                },
                LadderElement::ParallelGroup {
                    id: "pg".into(),
                    branches: vec![
                        vec![LadderElement::ContactNo {
                            id: "i1".into(),
                            address: Address::discrete(1),
                        }],
                        vec![LadderElement::ContactNo {
                            id: "i2".into(),
                            address: Address::discrete(2),
                        }],
                    ],
                },
                LadderElement::Coil {
                    id: "q".into(),
                    address: Address::coil(0),
                },
            ],
        );
        eng.load_program(compile(prog).unwrap());
        let mut s = ScanState::default();

        mem.set_discrete(0, true).unwrap(); // I0=1, I1=I2=0 ⇒ false
        s.scan(&eng, 20);
        assert!(!mem.get_coil(0).unwrap());

        mem.set_discrete(1, true).unwrap(); // I0=1, I1=1 ⇒ true
        s.scan(&eng, 20);
        assert!(mem.get_coil(0).unwrap());

        mem.set_discrete(0, false).unwrap(); // incoming false, I1=I2=1 ⇒ false
        mem.set_discrete(2, true).unwrap();
        s.scan(&eng, 20);
        assert!(
            !mem.get_coil(0).unwrap(),
            "inline OR group must AND with the incoming series power"
        );
    }

    // Deterministic fuzz: the scan path must NEVER panic — for any well-typed
    // program and any input state it returns Ok(tick) or a typed ScanError
    // (out-of-range address, overflow, divide-by-zero → FAULT/STOP).
    #[test]
    fn fuzz_scan_never_panics() {
        use crate::plc::compiler::{CmpOp, MathOp, MemArea};

        fn lcg(state: &mut u64) -> u64 {
            *state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *state
        }
        fn addr(r: &mut u64) -> Address {
            let area = match lcg(r) % 6 {
                0 => MemArea::Coil,
                1 => MemArea::Discrete,
                2 => MemArea::Holding,
                3 => MemArea::InputReg,
                4 => MemArea::MemoryBit,
                _ => MemArea::MemoryWord,
            };
            // Range spans well beyond process-image bounds to exercise the
            // out-of-range → ScanError::Memory path without panicking.
            let index = (lcg(r) % 6000) as u16;
            let bit = if lcg(r) % 2 == 0 {
                Some((lcg(r) % 16) as u8)
            } else {
                None
            };
            Address { area, index, bit }
        }
        fn maybe(r: &mut u64) -> Option<Address> {
            if lcg(r) % 2 == 0 {
                Some(addr(r))
            } else {
                None
            }
        }
        fn element(r: &mut u64) -> LadderElement {
            let id = format!("e{}", lcg(r) % 100_000);
            match lcg(r) % 12 {
                0 => LadderElement::ContactNo {
                    id,
                    address: addr(r),
                },
                1 => LadderElement::ContactNc {
                    id,
                    address: addr(r),
                },
                2 => LadderElement::ContactRising {
                    id,
                    address: addr(r),
                },
                3 => LadderElement::Coil {
                    id,
                    address: addr(r),
                },
                4 => LadderElement::CoilSet {
                    id,
                    address: addr(r),
                },
                5 => LadderElement::CoilReset {
                    id,
                    address: addr(r),
                },
                6 => LadderElement::Ton {
                    id,
                    preset_ms: (lcg(r) % 5000) as u32,
                    timer_index: (lcg(r) % 40) as u16,
                    done_address: maybe(r),
                },
                7 => LadderElement::Ctu {
                    id,
                    preset: (lcg(r) % 50) as u16,
                    counter_index: (lcg(r) % 40) as u16,
                    done_address: maybe(r),
                    reset_address: maybe(r),
                },
                8 => LadderElement::Math {
                    id,
                    op: match lcg(r) % 4 {
                        0 => MathOp::Add,
                        1 => MathOp::Sub,
                        2 => MathOp::Mul,
                        _ => MathOp::Div,
                    },
                    a: addr(r),
                    b: addr(r),
                    dest: addr(r),
                },
                9 => LadderElement::Move {
                    id,
                    source: addr(r),
                    dest: addr(r),
                },
                10 => LadderElement::Compare {
                    id,
                    op: CmpOp::Lt,
                    a: addr(r),
                    b: addr(r),
                },
                _ => LadderElement::Wire { id },
            }
        }

        let mut r: u64 = 0xD1B54A32D192ED03;
        for _ in 0..300 {
            let n = 1 + (lcg(&mut r) % 6) as usize;
            let elements: Vec<LadderElement> = (0..n).map(|_| element(&mut r)).collect();
            let prog = LadderProgram {
                name: "fuzz".into(),
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
            };
            let Ok(compiled) = compile(prog) else {
                continue;
            };
            let mem = PlcMemory::new().into_arc();
            let eng = PlcEngine::new(Arc::clone(&mem));
            eng.load_program(compiled);

            // Randomise inputs, then scan repeatedly. Any error is acceptable;
            // a panic (index-out-of-bounds, arithmetic panic, …) is not.
            for i in 0..(lcg(&mut r) % 4) as u16 {
                let _ = mem.set_discrete(i, lcg(&mut r) % 2 == 0);
                let _ = mem.set_holding(i, (lcg(&mut r) % 65536) as u16);
            }
            let mut timers = HashMap::new();
            let mut counters = HashMap::new();
            let mut edges = HashMap::new();
            for _ in 0..3 {
                // Result intentionally ignored: Ok or Err, but must not panic.
                let _ = eng.execute_scan(20, &mut timers, &mut counters, &mut edges);
            }
        }
    }
}
