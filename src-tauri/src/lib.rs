//! =============================================================================
//! PLC Ladder Simulator Pro — Tauri v2 application library entry.
//! =============================================================================

mod audit;
mod commands;
mod plc;

use audit::AuditTrail;
use commands::AppState;
use plc::compiler::{compile, demo_program};
use plc::engine::PlcEngine;
use plc::memory::PlcMemory;
use plc::modbus::ModbusController;
use plc::modbus_map::ModbusMap;
use plc::symbols::SymbolTable;
use std::sync::Arc;
use tauri::Manager;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,plc_ladder_sim_lib=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .init();

    info!("PLC Ladder Simulator Pro starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let memory = PlcMemory::new().into_arc();
            memory.set_cycle_ms(20);

            let engine = PlcEngine::new(Arc::clone(&memory));
            let audit = AuditTrail::new();
            let map = ModbusMap::new();
            let modbus = ModbusController::new(Arc::clone(&memory), Arc::clone(&map));
            let symbols = SymbolTable::new();

            if let Ok(dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&dir);
                audit.set_log_path(dir.join("audit_trail.jsonl"));
            }

            if let Ok(compiled) = compile(demo_program()) {
                engine.load_program(compiled);
            }

            audit.record(
                "system",
                "APPLICATION_START",
                "PLC Ladder Simulator Pro boot",
                memory.program_hash(),
            );

            // Modbus OFF by default — user enables from Device config (TIA-style)
            // Optional auto-start can be toggled from UI.
            info!(
                port = modbus.status().port,
                "Modbus controller ready (disabled until started)"
            );

            app.manage(AppState {
                memory,
                engine,
                audit,
                modbus,
                symbols,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::update_program,
            commands::load_demo_program,
            commands::start_simulation,
            commands::stop_simulation,
            commands::get_memory_snapshot,
            commands::get_status,
            commands::set_cycle_ms,
            commands::set_discrete_input,
            commands::set_coil,
            commands::set_holding_register,
            commands::export_program_json,
            commands::import_program_json,
            commands::export_program_bytecode,
            commands::import_program_bytecode,
            commands::reset_memory,
            commands::export_audit_report,
            commands::get_audit_entries,
            commands::verify_audit_chain,
            commands::get_program,
            commands::get_active_elements,
            commands::get_symbols,
            commands::set_symbols,
            commands::upsert_symbol,
            commands::remove_symbol,
            commands::get_modbus_status,
            commands::start_modbus,
            commands::stop_modbus,
            commands::set_modbus_port,
            commands::get_modbus_map,
            commands::set_modbus_map,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PLC Ladder Simulator Pro");
}
