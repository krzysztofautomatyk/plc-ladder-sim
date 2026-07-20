//! =============================================================================
//! Tauri Commands — IPC surface for the frontend.
//! update_program, start/stop_simulation, get_memory_snapshot, Modbus/IO helpers.
//! =============================================================================

use crate::audit::{AuditEntry, AuditReport, AuditTrail};
use crate::plc::compiler::{
    compile, demo_program, export_bytecode, import_bytecode, LadderProgram,
};
use crate::plc::engine::PlcEngine;
use crate::plc::memory::{MemorySnapshot, PlcMemory, PlcRunState};
use crate::plc::modbus::{ModbusController, ModbusStatus};
use crate::plc::modbus_map::ModbusMapSnapshot;
use crate::plc::symbols::{PlcSymbol, SymbolTable, SymbolTableSnapshot};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;
use tracing::info;

pub struct AppState {
    pub memory: Arc<PlcMemory>,
    pub engine: Arc<PlcEngine>,
    pub audit: Arc<AuditTrail>,
    pub modbus: Arc<ModbusController>,
    pub symbols: Arc<SymbolTable>,
}

#[derive(Debug, Serialize)]
pub struct CommandResult<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> CommandResult<T> {
    fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProgramInfo {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub instruction_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SimStatus {
    pub running: bool,
    pub run_state: PlcRunState,
    pub cycle_ms: u32,
    pub scan_count: u64,
    pub last_scan_us: u64,
    pub program_hash: String,
    pub program_version: String,
    pub modbus_port: u16,
    pub modbus_enabled: bool,
    pub modbus_running: bool,
    pub modbus_error: String,
    pub fault_code: u16,
    pub fault_message: String,
}

/// Compile and load ladder program JSON into the engine.
#[tauri::command]
pub fn update_program(
    state: State<'_, AppState>,
    program: LadderProgram,
) -> CommandResult<ProgramInfo> {
    match compile(program) {
        Ok(compiled) => {
            let info = ProgramInfo {
                name: compiled.name.clone(),
                version: compiled.version.clone(),
                hash: compiled.hash.clone(),
                instruction_count: compiled.instructions.len(),
            };
            state.audit.record(
                "operator",
                "UPDATE_PROGRAM",
                format!(
                    "name={} version={} hash={} instr={}",
                    info.name, info.version, info.hash, info.instruction_count
                ),
                &info.hash,
            );
            state.engine.load_program(compiled);
            info!(hash = %info.hash, "program updated");
            CommandResult::ok(info)
        }
        Err(e) => {
            state.audit.record(
                "operator",
                "UPDATE_PROGRAM_FAILED",
                e.to_string(),
                state.memory.program_hash(),
            );
            CommandResult::err(e.to_string())
        }
    }
}

/// Load built-in demo program.
#[tauri::command]
pub fn load_demo_program(state: State<'_, AppState>) -> CommandResult<ProgramInfo> {
    let program = demo_program();
    update_program(state, program)
}

/// Start PLC scan cycle simulation.
#[tauri::command]
pub fn start_simulation(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<SimStatus> {
    if state.engine.is_running() {
        return CommandResult::err("simulation already running");
    }
    match state.engine.start(app) {
        Ok(()) => {
            state.audit.record(
                "operator",
                "START_SIMULATION",
                format!("cycle_ms={}", state.memory.cycle_ms()),
                state.memory.program_hash(),
            );
            CommandResult::ok(current_status(&state))
        }
        Err(e) => {
            state.audit.record(
                "operator",
                "START_SIMULATION_FAILED",
                e.to_string(),
                state.memory.program_hash(),
            );
            CommandResult::err(e.to_string())
        }
    }
}

/// Stop PLC scan cycle.
#[tauri::command]
pub fn stop_simulation(state: State<'_, AppState>) -> CommandResult<SimStatus> {
    state.engine.stop();
    state.audit.record(
        "operator",
        "STOP_SIMULATION",
        format!("scan_count={}", state.memory.scan_count()),
        state.memory.program_hash(),
    );
    // Brief wait for thread to notice — non-blocking from UI perspective
    CommandResult::ok(current_status(&state))
}

/// Full or compact process image snapshot.
#[tauri::command]
pub fn get_memory_snapshot(
    state: State<'_, AppState>,
    compact: Option<bool>,
) -> CommandResult<MemorySnapshot> {
    let snap = if compact.unwrap_or(true) {
        // Include MW40+ used by demo compare/move and timer/counter pairs
        state.memory.snapshot_compact(128, 64)
    } else {
        state.memory.snapshot()
    };
    CommandResult::ok(snap)
}

/// Runtime status for toolbar / SCADA diagnostics.
#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> CommandResult<SimStatus> {
    CommandResult::ok(current_status(&state))
}

/// Configure scan cycle (5–100 ms).
#[tauri::command]
pub fn set_cycle_ms(state: State<'_, AppState>, cycle_ms: u32) -> CommandResult<u32> {
    state.memory.set_cycle_ms(cycle_ms);
    let actual = state.memory.cycle_ms();
    state.audit.record(
        "operator",
        "SET_CYCLE_MS",
        format!("{actual}"),
        state.memory.program_hash(),
    );
    CommandResult::ok(actual)
}

/// Force discrete input (simulated field sensor) from UI.
#[tauri::command]
pub fn set_discrete_input(
    state: State<'_, AppState>,
    address: u16,
    value: bool,
) -> CommandResult<bool> {
    match state.memory.set_discrete(address, value) {
        Ok(()) => {
            state.audit.record(
                "operator",
                "SET_DISCRETE",
                format!("I{address}={value}"),
                state.memory.program_hash(),
            );
            CommandResult::ok(value)
        }
        Err(e) => CommandResult::err(e.to_string()),
    }
}

/// Force coil (manual override / HMI).
#[tauri::command]
pub fn set_coil(state: State<'_, AppState>, address: u16, value: bool) -> CommandResult<bool> {
    match state.memory.set_coil(address, value) {
        Ok(()) => CommandResult::ok(value),
        Err(e) => CommandResult::err(e.to_string()),
    }
}

/// Write holding register from UI.
#[tauri::command]
pub fn set_holding_register(
    state: State<'_, AppState>,
    address: u16,
    value: u16,
) -> CommandResult<u16> {
    match state.memory.set_holding(address, value) {
        Ok(()) => CommandResult::ok(value),
        Err(e) => CommandResult::err(e.to_string()),
    }
}

/// Export program as JSON string.
#[tauri::command]
pub fn export_program_json(state: State<'_, AppState>) -> CommandResult<String> {
    match state.engine.program_snapshot() {
        Some(p) => match serde_json::to_string_pretty(&p.source) {
            Ok(s) => {
                state.audit.record(
                    "operator",
                    "EXPORT_JSON",
                    format!("bytes={}", s.len()),
                    p.hash,
                );
                CommandResult::ok(s)
            }
            Err(e) => CommandResult::err(e.to_string()),
        },
        None => CommandResult::err("no program loaded"),
    }
}

/// Import program from JSON string.
#[tauri::command]
pub fn import_program_json(state: State<'_, AppState>, json: String) -> CommandResult<ProgramInfo> {
    match serde_json::from_str::<LadderProgram>(&json) {
        Ok(program) => update_program(state, program),
        Err(e) => CommandResult::err(format!("invalid JSON: {e}")),
    }
}

/// Export compiled bytecode as base64.
#[tauri::command]
pub fn export_program_bytecode(state: State<'_, AppState>) -> CommandResult<String> {
    match state.engine.program_snapshot() {
        Some(p) => match export_bytecode(&p) {
            Ok(bytes) => {
                // Hex-encode the compiled bincode bytecode. Integrity is anchored
                // by the program's SHA-256 hash carried inside the package.
                let hex = hex::encode(&bytes);
                state.audit.record(
                    "operator",
                    "EXPORT_BYTECODE",
                    format!("len={}", bytes.len()),
                    p.hash,
                );
                CommandResult::ok(hex)
            }
            Err(e) => CommandResult::err(e.to_string()),
        },
        None => CommandResult::err("no program loaded"),
    }
}

/// Import bytecode from hex string.
#[tauri::command]
pub fn import_program_bytecode(
    state: State<'_, AppState>,
    hex_data: String,
) -> CommandResult<ProgramInfo> {
    match hex::decode(hex_data.trim()) {
        Ok(bytes) => match import_bytecode(&bytes) {
            Ok(compiled) => {
                // Re-compile from source for consistency of debug map
                match compile(compiled.source) {
                    Ok(c) => {
                        let info = ProgramInfo {
                            name: c.name.clone(),
                            version: c.version.clone(),
                            hash: c.hash.clone(),
                            instruction_count: c.instructions.len(),
                        };
                        state.engine.load_program(c);
                        state.audit.record(
                            "operator",
                            "IMPORT_BYTECODE",
                            format!("hash={}", info.hash),
                            &info.hash,
                        );
                        CommandResult::ok(info)
                    }
                    Err(e) => CommandResult::err(e.to_string()),
                }
            }
            Err(e) => CommandResult::err(e.to_string()),
        },
        Err(e) => CommandResult::err(format!("hex decode: {e}")),
    }
}

/// Reset process image (I/O and registers).
#[tauri::command]
pub fn reset_memory(state: State<'_, AppState>) -> CommandResult<MemorySnapshot> {
    if state.engine.is_running() {
        return CommandResult::err("stop simulation before reset");
    }
    state.memory.reset_process_image();
    state.audit.record(
        "operator",
        "RESET_MEMORY",
        "process image cleared",
        state.memory.program_hash(),
    );
    CommandResult::ok(state.memory.snapshot_compact(128, 64))
}

/// Export audit report JSON.
#[tauri::command]
pub fn export_audit_report(state: State<'_, AppState>) -> CommandResult<AuditReport> {
    let report = state.audit.report();
    state.audit.record(
        "operator",
        "EXPORT_AUDIT",
        format!(
            "entries={} valid={}",
            report.entry_count, report.chain_valid
        ),
        state.memory.program_hash(),
    );
    CommandResult::ok(state.audit.report())
}

/// List recent audit entries.
#[tauri::command]
pub fn get_audit_entries(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> CommandResult<Vec<AuditEntry>> {
    let mut entries = state.audit.entries();
    if let Some(n) = limit {
        let skip = entries.len().saturating_sub(n);
        entries = entries.into_iter().skip(skip).collect();
    }
    CommandResult::ok(entries)
}

/// Verify audit hash chain integrity.
#[tauri::command]
pub fn verify_audit_chain(state: State<'_, AppState>) -> CommandResult<bool> {
    CommandResult::ok(state.audit.verify_chain())
}

/// Return currently loaded compiled program (source AST for editor).
#[tauri::command]
pub fn get_program(state: State<'_, AppState>) -> CommandResult<Option<LadderProgram>> {
    CommandResult::ok(state.engine.program_snapshot().map(|p| p.source))
}

/// Active element IDs from last scan (for offline poll fallback).
#[tauri::command]
pub fn get_active_elements(state: State<'_, AppState>) -> CommandResult<Vec<String>> {
    CommandResult::ok(state.engine.last_active_elements())
}

// ─── Symbol table ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_symbols(state: State<'_, AppState>) -> CommandResult<SymbolTableSnapshot> {
    CommandResult::ok(state.symbols.snapshot())
}

#[tauri::command]
pub fn set_symbols(
    state: State<'_, AppState>,
    symbols: Vec<PlcSymbol>,
) -> CommandResult<SymbolTableSnapshot> {
    state.symbols.set_all(symbols);
    state.audit.record(
        "operator",
        "SET_SYMBOLS",
        format!("count={}", state.symbols.list().len()),
        state.memory.program_hash(),
    );
    CommandResult::ok(state.symbols.snapshot())
}

#[tauri::command]
pub fn upsert_symbol(
    state: State<'_, AppState>,
    symbol: PlcSymbol,
) -> CommandResult<SymbolTableSnapshot> {
    state.symbols.upsert(symbol);
    CommandResult::ok(state.symbols.snapshot())
}

#[tauri::command]
pub fn remove_symbol(state: State<'_, AppState>, id: String) -> CommandResult<SymbolTableSnapshot> {
    state.symbols.remove(&id);
    CommandResult::ok(state.symbols.snapshot())
}

// ─── Modbus control & map ───────────────────────────────────────────────────

#[tauri::command]
pub fn get_modbus_status(state: State<'_, AppState>) -> CommandResult<ModbusStatus> {
    CommandResult::ok(state.modbus.status())
}

#[tauri::command]
pub fn start_modbus(state: State<'_, AppState>) -> CommandResult<ModbusStatus> {
    match state.modbus.start() {
        Ok(st) => {
            state.audit.record(
                "operator",
                "MODBUS_START",
                format!("port={}", st.port),
                state.memory.program_hash(),
            );
            CommandResult::ok(st)
        }
        Err(e) => {
            state.audit.record(
                "operator",
                "MODBUS_START_FAILED",
                e.clone(),
                state.memory.program_hash(),
            );
            CommandResult::err(e)
        }
    }
}

#[tauri::command]
pub fn stop_modbus(state: State<'_, AppState>) -> CommandResult<ModbusStatus> {
    let st = state.modbus.stop();
    state.audit.record(
        "operator",
        "MODBUS_STOP",
        format!("port={}", st.port),
        state.memory.program_hash(),
    );
    CommandResult::ok(st)
}

#[tauri::command]
pub fn set_modbus_port(state: State<'_, AppState>, port: u16) -> CommandResult<ModbusStatus> {
    match state.modbus.set_port(port) {
        Ok(()) => {
            state.audit.record(
                "operator",
                "MODBUS_SET_PORT",
                format!("{port}"),
                state.memory.program_hash(),
            );
            CommandResult::ok(state.modbus.status())
        }
        Err(e) => CommandResult::err(e),
    }
}

#[tauri::command]
pub fn set_modbus_write_enabled(
    state: State<'_, AppState>,
    allow: bool,
) -> CommandResult<ModbusStatus> {
    state.memory.set_allow_modbus_write(allow);
    state.audit.record(
        "operator",
        "MODBUS_SET_WRITE_ENABLED",
        format!("{allow}"),
        state.memory.program_hash(),
    );
    CommandResult::ok(state.modbus.status())
}

#[tauri::command]
pub fn get_modbus_map(state: State<'_, AppState>) -> CommandResult<ModbusMapSnapshot> {
    CommandResult::ok(state.modbus.map().snapshot())
}

#[tauri::command]
pub fn set_modbus_map(
    state: State<'_, AppState>,
    map: ModbusMapSnapshot,
) -> CommandResult<ModbusMapSnapshot> {
    let n = map.entries.len();
    state.modbus.map().set_all(map);
    state.audit.record(
        "operator",
        "MODBUS_SET_MAP",
        format!("entries={n}"),
        state.memory.program_hash(),
    );
    CommandResult::ok(state.modbus.map().snapshot())
}

fn current_status(state: &AppState) -> SimStatus {
    let mb = state.modbus.status();
    SimStatus {
        running: state.engine.is_running(),
        run_state: state.memory.run_state(),
        cycle_ms: state.memory.cycle_ms(),
        scan_count: state.memory.scan_count(),
        last_scan_us: state.memory.last_scan_us(),
        program_hash: state.memory.program_hash(),
        program_version: state.memory.program_version(),
        modbus_port: mb.port,
        modbus_enabled: mb.enabled,
        modbus_running: mb.running,
        modbus_error: mb.last_error,
        fault_code: state.memory.fault_code(),
        fault_message: state.memory.snapshot().fault_message,
    }
}
