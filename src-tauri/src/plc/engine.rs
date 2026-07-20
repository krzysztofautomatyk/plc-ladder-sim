//! =============================================================================
//! PLC Scan Engine — deterministic cyclic execution on a dedicated Tokio task.
//! High thread priority, checked arithmetic, faults → STOP (no panic).
//! Emits live debug events for active element highlighting.
//! =============================================================================

use super::compiler::{CmpOp, CompiledProgram, Instruction, MathOp, MemArea};
use super::memory::{PlcMemory, PlcRunState};
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

    pub fn memory(&self) -> Arc<PlcMemory> {
        Arc::clone(&self.memory)
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
        /// Previous bit level per edge-contact element id (rising/falling)
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
                    // Compact memory event every scan for live table
                    let snap = self.memory.snapshot_compact(64, 32);
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
        let mut active_elements: HashSet<String> = HashSet::new();
        let mut active_rungs: HashSet<String> = HashSet::new();
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
                    or_accum = false;
                    power = true;
                }
                Instruction::OrAlt => {
                    or_accum |= power;
                    power = true;
                }
                Instruction::OrEnd => {
                    or_accum |= power;
                    power = or_accum;
                }
                Instruction::LoadNo { area, index, bit } => {
                    let v = self.read_bit(*area, *index, *bit)?;
                    power = power && v;
                    if let Some(id) = inv_debug.get(&i) {
                        if power {
                            active_elements.insert(id.clone());
                        }
                    }
                }
                Instruction::LoadNc { area, index, bit } => {
                    let v = self.read_bit(*area, *index, *bit)?;
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
                    let now = self.read_bit(*area, *index, *bit)?;
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
                    let now = self.read_bit(*area, *index, *bit)?;
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
                    let latched = self.read_bit(*area, *index, *bit)?;
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
                        self.read_bit(*a, *idx, *bit)?
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
                        self.read_bit(*a, *idx, *bit)?
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
                        self.read_bit(*a, *idx, *bit)?
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
                        let va = self.read_word(a.0, a.1)?;
                        let vb = self.read_word(b.0, b.1)?;
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
                        let v = self.read_word(source.0, source.1)?;
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
                    let va = self.read_word(a.0, a.1)?;
                    let vb = self.read_word(b.0, b.1)?;
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
        let base = timer_index.saturating_mul(2);
        self.memory
            .set_holding(base, st.et_ms.min(u16::MAX as u32) as u16)
            .map_err(|e| ScanError::Memory(e.to_string()))?;
        self.memory
            .set_holding(base.saturating_add(1), if st.q { 1 } else { 0 })
            .map_err(|e| ScanError::Memory(e.to_string()))
    }

    fn write_counter_regs(&self, counter_index: u16, st: &CounterState) -> Result<(), ScanError> {
        let base = counter_index.saturating_mul(2);
        self.memory
            .set_holding(base, st.cv)
            .map_err(|e| ScanError::Memory(e.to_string()))?;
        self.memory
            .set_holding(base.saturating_add(1), if st.q { 1 } else { 0 })
            .map_err(|e| ScanError::Memory(e.to_string()))
    }

    fn read_bit(
        &self,
        area: MemArea,
        index: u16,
        bit: Option<u8>,
    ) -> Result<bool, ScanError> {
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
        }
    }

    fn read_word(&self, area: MemArea, index: u16) -> Result<u16, ScanError> {
        match area {
            MemArea::Holding => self
                .memory
                .get_holding(index)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::InputReg => self
                .memory
                .get_input_reg(index)
                .map_err(|e| ScanError::Memory(e.to_string())),
            MemArea::Coil => {
                let b = self
                    .memory
                    .get_coil(index)
                    .map_err(|e| ScanError::Memory(e.to_string()))?;
                Ok(if b { 1 } else { 0 })
            }
            MemArea::Discrete => {
                let b = self
                    .memory
                    .get_discrete(index)
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
    use crate::plc::compiler::{compile, demo_program, Address, LadderElement, LadderProgram, Rung};
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
            eng.execute_scan(20, &mut timers, &mut counters, &mut edges).unwrap();
        }
        let et_mid = mem.get_holding(10).unwrap(); // timer 5 * 2
        assert!(et_mid >= 60);
        mem.set_discrete(0, false).unwrap();
        eng.execute_scan(20, &mut timers, &mut counters, &mut edges).unwrap();
        let et_hold = mem.get_holding(10).unwrap();
        assert_eq!(et_hold, et_mid, "RTO must retain ET when IN drops");
    }
}
