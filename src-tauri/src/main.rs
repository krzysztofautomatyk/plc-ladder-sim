//! =============================================================================
//! PLC Ladder Simulator Pro — binary entry point (desktop).
//! =============================================================================

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    plc_ladder_sim_lib::run();
}
