/**
 * Tauri IPC helpers + browser mock fallback.
 */
import type {
  AuditEntry,
  AuditReport,
  CommandResult,
  LadderProgram,
  MemorySnapshot,
  ModbusMapSnapshot,
  ModbusStatus,
  PlcSymbol,
  ProgramInfo,
  SimStatus,
  SymbolTableSnapshot,
} from "./types";

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<CommandResult<T>> {
  try {
    const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
    return await tauriInvoke<CommandResult<T>>(cmd, args);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("not available") || msg.includes("__TAURI__")) {
      return mockInvoke<T>(cmd, args);
    }
    return { ok: false, error: msg };
  }
}

function mockInvoke<T>(cmd: string, _args?: Record<string, unknown>): CommandResult<T> {
  console.warn(`[mock] ${cmd}`, _args);
  if (cmd === "get_status") {
    return {
      ok: true,
      data: {
        running: false,
        run_state: "stop",
        cycle_ms: 20,
        scan_count: 0,
        last_scan_us: 0,
        program_hash: "mock",
        program_version: "0.0.0",
        modbus_port: 5020,
        modbus_enabled: false,
        modbus_running: false,
        modbus_error: "",
        fault_code: 0,
        fault_message: "",
      } as T,
    };
  }
  if (cmd === "get_modbus_status") {
    return {
      ok: true,
      data: {
        enabled: false,
        running: false,
        port: 5020,
        bind: "127.0.0.1",
        write_enabled: false,
        last_error: "",
      } as T,
    };
  }
  if (cmd === "get_symbols") {
    return { ok: true, data: { symbols: [] } as T };
  }
  if (cmd === "get_modbus_map") {
    return { ok: true, data: { entries: [], identity_fallback: true } as T };
  }
  if (cmd === "get_memory_snapshot") {
    return {
      ok: true,
      data: {
        coils: Array(64).fill(false),
        discrete_inputs: Array(64).fill(false),
        holding_registers: Array(64).fill(0),
        input_registers: Array(16).fill(0),
        run_state: "stop",
        scan_count: 0,
        last_scan_us: 0,
        cycle_ms: 20,
        program_hash: "mock",
        program_version: "0.0.0",
        fault_code: 0,
        fault_message: "",
      } as T,
    };
  }
  return { ok: true, data: null as T };
}

export const api = {
  updateProgram: (program: LadderProgram) =>
    invoke<ProgramInfo>("update_program", { program }),
  loadDemo: () => invoke<ProgramInfo>("load_demo_program"),
  start: () => invoke<SimStatus>("start_simulation"),
  stop: () => invoke<SimStatus>("stop_simulation"),
  getMemory: (compact = true) =>
    invoke<MemorySnapshot>("get_memory_snapshot", { compact }),
  getStatus: () => invoke<SimStatus>("get_status"),
  setCycleMs: (cycle_ms: number) => invoke<number>("set_cycle_ms", { cycle_ms }),
  setDiscrete: (address: number, value: boolean) =>
    invoke<boolean>("set_discrete_input", { address, value }),
  setCoil: (address: number, value: boolean) =>
    invoke<boolean>("set_coil", { address, value }),
  setHolding: (address: number, value: number) =>
    invoke<number>("set_holding_register", { address, value }),
  exportJson: () => invoke<string>("export_program_json"),
  importJson: (json: string) => invoke<ProgramInfo>("import_program_json", { json }),
  exportBytecode: () => invoke<string>("export_program_bytecode"),
  importBytecode: (hex_data: string) =>
    invoke<ProgramInfo>("import_program_bytecode", { hex_data }),
  resetMemory: () => invoke<MemorySnapshot>("reset_memory"),
  getProgram: () => invoke<LadderProgram | null>("get_program"),
  getAudit: (limit = 50) => invoke<AuditEntry[]>("get_audit_entries", { limit }),
  verifyAudit: () => invoke<boolean>("verify_audit_chain"),
  exportAudit: () => invoke<AuditReport>("export_audit_report"),
  getSymbols: () => invoke<SymbolTableSnapshot>("get_symbols"),
  setSymbols: (symbols: PlcSymbol[]) => invoke<SymbolTableSnapshot>("set_symbols", { symbols }),
  upsertSymbol: (symbol: PlcSymbol) =>
    invoke<SymbolTableSnapshot>("upsert_symbol", { symbol }),
  removeSymbol: (id: string) => invoke<SymbolTableSnapshot>("remove_symbol", { id }),
  getModbusStatus: () => invoke<ModbusStatus>("get_modbus_status"),
  startModbus: () => invoke<ModbusStatus>("start_modbus"),
  stopModbus: () => invoke<ModbusStatus>("stop_modbus"),
  setModbusPort: (port: number) => invoke<ModbusStatus>("set_modbus_port", { port }),
  setModbusWriteEnabled: (allow: boolean) =>
    invoke<ModbusStatus>("set_modbus_write_enabled", { allow }),
  getModbusMap: () => invoke<ModbusMapSnapshot>("get_modbus_map"),
  setModbusMap: (map: ModbusMapSnapshot) =>
    invoke<ModbusMapSnapshot>("set_modbus_map", { map }),
};

export async function listenScanTick(
  handler: (payload: unknown) => void
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const un = await listen("plc://scan-tick", (e) => handler(e.payload));
    return () => un();
  } catch {
    return () => {};
  }
}

export async function listenMemory(
  handler: (payload: unknown) => void
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const un = await listen("plc://memory", (e) => handler(e.payload));
    return () => un();
  } catch {
    return () => {};
  }
}

export async function listenFault(
  handler: (payload: unknown) => void
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const un = await listen("plc://fault", (e) => handler(e.payload));
    return () => un();
  } catch {
    return () => {};
  }
}
