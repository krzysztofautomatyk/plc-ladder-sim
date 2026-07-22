/** Shared TypeScript types mirroring the Rust PLC API. */

export type MemArea =
  | "coil"
  | "discrete"
  | "holding"
  | "input_reg"
  | "memory_bit"
  | "memory_word";
export type CmpOp = "eq" | "ne" | "gt" | "ge" | "lt" | "le";
export type MathOp = "add" | "sub" | "mul" | "div";
export type DataType = "bool" | "word" | "int" | "d_int";
export type ModbusTable = "coil" | "discrete" | "holding" | "input_reg";
/** How a Modbus matrix rule translates PLC ↔ Modbus. */
export type MappingType = "direct" | "bit_to_register" | "register_to_bit";
/** Per-rule write-protect deny behaviour (global SCADA write enable still required). */
export type WriteProtectMode = "strict" | "silent_drop";

export interface Address {
  area: MemArea;
  index: number;
  /** Bit 0–15 inside a holding/input word (e.g. R1.3). */
  bit?: number | null;
}

export type LadderElement =
  | { type: "contact_no"; id: string; address: Address }
  | { type: "contact_nc"; id: string; address: Address }
  | { type: "contact_rising"; id: string; address: Address }
  | { type: "contact_falling"; id: string; address: Address }
  | { type: "coil"; id: string; address: Address }
  | { type: "coil_negated"; id: string; address: Address }
  | { type: "coil_set"; id: string; address: Address }
  | { type: "coil_reset"; id: string; address: Address }
  | {
      type: "ton";
      id: string;
      preset_ms: number;
      timer_index: number;
      done_address?: Address | null;
    }
  | {
      type: "tof";
      id: string;
      preset_ms: number;
      timer_index: number;
      done_address?: Address | null;
    }
  | {
      type: "rto";
      id: string;
      preset_ms: number;
      timer_index: number;
      done_address?: Address | null;
      reset_address?: Address | null;
    }
  | {
      type: "ctu";
      id: string;
      preset: number;
      counter_index: number;
      done_address?: Address | null;
      reset_address?: Address | null;
    }
  | {
      type: "ctd";
      id: string;
      preset: number;
      counter_index: number;
      done_address?: Address | null;
      load_address?: Address | null;
    }
  | {
      type: "math";
      id: string;
      op: MathOp;
      a: Address;
      b: Address;
      dest: Address;
    }
  | { type: "move"; id: string; source: Address; dest: Address }
  | { type: "compare"; id: string; op: CmpOp; a: Address; b: Address }
  | { type: "wire"; id: string };

/** A parallel OR group placed inline within a rung's series (branches of contacts). */
export interface ParallelNode {
  type: "parallel";
  id: string;
  branches: LadderElement[][];
}

/** A node in a rung's series: a normal element or an inline parallel group. */
export type RungNode = LadderElement | ParallelNode;

export interface Rung {
  id: string;
  comment: string;
  elements: RungNode[];
  or_branches: LadderElement[][];
  enabled: boolean;
}

export interface LadderProgram {
  name: string;
  version: string;
  description: string;
  rungs: Rung[];
  metadata: Record<string, string>;
}

export type PlcRunState = "stop" | "run" | "fault";

export interface MemorySnapshot {
  coils: boolean[];
  discrete_inputs: boolean[];
  holding_registers: number[];
  input_registers: number[];
  memory_bits: boolean[];
  memory_words: number[];
  /** Timer ET (ms) by instance — always full bank in live snapshots. */
  timer_et?: number[];
  /** Timer done Q by instance. */
  timer_q?: boolean[];
  /** Counter CV by instance. */
  counter_cv?: number[];
  /** Counter done Q by instance. */
  counter_q?: boolean[];
  run_state: PlcRunState;
  scan_count: number;
  last_scan_us: number;
  cycle_ms: number;
  program_hash: string;
  program_version: string;
  fault_code: number;
  fault_message: string;
}

export interface SimStatus {
  running: boolean;
  run_state: PlcRunState;
  cycle_ms: number;
  scan_count: number;
  last_scan_us: number;
  program_hash: string;
  program_version: string;
  modbus_port: number;
  modbus_enabled: boolean;
  modbus_running: boolean;
  modbus_error: string;
  fault_code: number;
  fault_message: string;
}

export interface ProgramInfo {
  name: string;
  version: string;
  hash: string;
  instruction_count: number;
}

export interface ScanTickEvent {
  scan_count: number;
  last_scan_us: number;
  cycle_ms: number;
  run_state: PlcRunState;
  active_elements: string[];
  active_rungs: string[];
  fault_code: number;
  fault_message: string;
}

export interface CommandResult<T> {
  ok: boolean;
  data?: T | null;
  error?: string | null;
}

export interface AuditEntry {
  id: string;
  sequence: number;
  timestamp: string;
  actor: string;
  action: string;
  detail: string;
  program_hash: string;
  prev_hash: string;
  entry_hash: string;
}

export interface AuditReport {
  generated_at: string;
  entry_count: number;
  chain_valid: boolean;
  head_hash: string;
  entries: AuditEntry[];
}

export interface PlcSymbol {
  id: string;
  name: string;
  area: MemArea;
  index: number;
  data_type: DataType;
  comment: string;
  address_display: string;
}

export interface SymbolTableSnapshot {
  symbols: PlcSymbol[];
}

export interface ModbusStatus {
  enabled: boolean;
  running: boolean;
  port: number;
  bind: string;
  write_enabled: boolean;
  last_error: string;
}

export interface ModbusMapEntry {
  id: string;
  enabled: boolean;
  /** Default: direct (1:1). */
  mapping_type?: MappingType;
  symbol_name: string;
  plc_area: MemArea;
  /** PLC start index (serde alias of legacy plc_index). */
  plc_start: number;
  /** Bit offset 0–15 inside a word (RegisterToBit). */
  plc_bit_offset?: number;
  modbus_table: ModbusTable;
  /** First Modbus address (serde alias of legacy modbus_address). */
  modbus_start: number;
  /** Span on Modbus side (Direct ≥1; BitToRegister 1; RegisterToBit 16). */
  length?: number;
  /** When true, Modbus writes targeting this rule are denied. */
  is_write_protected?: boolean;
  comment: string;
}

export interface ModbusMapSnapshot {
  entries: ModbusMapEntry[];
  identity_fallback: boolean;
  /** Strict exception vs silent ACK when a rule is write-protected. */
  write_protect_mode?: WriteProtectMode;
}

export interface LogEntry {
  seq: number;
  ts: string;
  level: string;
  target: string;
  message: string;
  fields: string;
}

export interface MemoryConfig {
  inputs: number;
  outputs: number;
  markers: number;
  data16: number;
  data32: number;
  internal16: number;
  timers: number;
  counters: number;
}

export interface MemoryConfigInfo {
  config: MemoryConfig;
  limits: MemoryConfig;
  register_pool: number;
}

export type AppView =
  | "ladder"
  | "tags"
  | "modbus"
  | "regmap"
  | "math"
  | "audit"
  | "watch"
  | "logs"
  | "memory"
  | "alloc";

export type PaletteKind =
  | "contact_no"
  | "contact_nc"
  | "contact_rising"
  | "contact_falling"
  | "coil"
  | "coil_negated"
  | "coil_set"
  | "coil_reset"
  | "ton"
  | "tof"
  | "rto"
  | "ctu"
  | "ctd"
  | "math"
  | "move"
  | "compare"
  | "wire"
  | "or_branch";
