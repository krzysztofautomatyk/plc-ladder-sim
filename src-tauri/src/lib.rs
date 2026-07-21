//! =============================================================================
//! PLC Ladder Simulator Pro — Tauri v2 application library entry.
//! =============================================================================
#![forbid(unsafe_code)]

mod audit;
mod commands;
mod logbuf;
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
        .with(logbuf::layer())
        .init();

    info!("PLC Ladder Simulator Pro starting");

    tauri::Builder::default()
        .setup(|app| {
            let memory = PlcMemory::new().into_arc();
            memory.set_cycle_ms(20);

            let engine = PlcEngine::new(Arc::clone(&memory));
            let audit = AuditTrail::new();
            let map = ModbusMap::new();
            let modbus = ModbusController::new(Arc::clone(&memory), Arc::clone(&map));
            let symbols = SymbolTable::new();
            let mem_config = plc::memconfig::MemoryConfigStore::new();

            if let Ok(dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&dir);
                audit.set_log_path(dir.join("audit_trail.jsonl"));
            }

            // Restore the persisted hash chain so tamper-evidence spans restarts.
            let (restored, chain_intact) = audit.load_persisted();
            if restored > 0 && !chain_intact {
                audit.record(
                    "system",
                    "AUDIT_CHAIN_WARNING",
                    "restored audit trail failed integrity verification",
                    memory.program_hash(),
                );
            }

            if let Ok(compiled) = compile(demo_program()) {
                engine.load_program(compiled);
            }

            audit.record(
                "system",
                "APPLICATION_START",
                format!("PLC Ladder Simulator Pro boot (restored={restored}, chain_intact={chain_intact})"),
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
                mem_config,
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
            commands::set_memory_bit,
            commands::set_memory_word,
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
            commands::set_modbus_write_enabled,
            commands::get_modbus_map,
            commands::set_modbus_map,
            commands::get_logs,
            commands::clear_logs,
            commands::get_memory_config,
            commands::set_memory_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PLC Ladder Simulator Pro");
}
