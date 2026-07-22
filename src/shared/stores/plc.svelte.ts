/**
 * Reactive PLC application store (Svelte 5 runes).
 */
import { api, listenFault, listenMemory, listenScanTick } from "../lib/api";
import { createDemoProgram, uid } from "../lib/demoProgram";
import {
  createWaterTankProgram,
  createWaterTankSymbols,
  WATER_TANK_MEMORY_SEED,
} from "../lib/waterTankProgram";
import { createWaterTankModbusMap } from "../lib/waterTankModbusMap";
import { createElement } from "../../features/ladder/elements";
import {
  addParallelBranch,
  addToParallelBranch,
  insertBeforeCoils,
  makeParallelGroup,
  removeNodeById,
  removeParallelBranch as removeParallelBranchOp,
  updateNodeById,
} from "../../features/ladder/lib/ladderEdit";
import {
  canMoveInRung,
  findInRung,
  moveInRung,
  type MoveDir,
} from "../../features/ladder/lib/ladderMove";
import { SvelteSet } from "svelte/reactivity";
import type {
  Address,
  AppView,
  LadderElement,
  LadderProgram,
  LogEntry,
  MemoryConfig,
  MemorySnapshot,
  ModbusMapSnapshot,
  ModbusStatus,
  PaletteKind,
  PlcSymbol,
  Rung,
  ScanTickEvent,
  SimStatus,
} from "../lib/types";

function emptyMemory(): MemorySnapshot {
  // Match backend COMPACT_* sizes (R0…R255 for live MOVE/math/cmp on water-tank).
  return {
    coils: Array(256).fill(false),
    discrete_inputs: Array(256).fill(false),
    holding_registers: Array(256).fill(0),
    input_registers: Array(32).fill(0),
    memory_bits: Array(256).fill(false),
    memory_words: Array(256).fill(0),
    timer_et: Array(256).fill(0),
    timer_q: Array(256).fill(false),
    counter_cv: Array(256).fill(0),
    counter_q: Array(256).fill(false),
    run_state: "stop",
    scan_count: 0,
    last_scan_us: 0,
    cycle_ms: 20,
    program_hash: "",
    program_version: "",
    fault_code: 0,
    fault_message: "",
  };
}

function ensureOr(r: Rung): Rung {
  return { ...r, or_branches: r.or_branches ?? [] };
}

/** Replace the contents of a reactive set in place (keeps the same instance). */
function syncSet(target: SvelteSet<string>, values: string[]): void {
  target.clear();
  for (const v of values) target.add(v);
}

class PlcStore {
  /** Default project: dual-pump wet-well (not the small 4-rung demo). */
  program = $state<LadderProgram>(createWaterTankProgram());
  memory = $state<MemorySnapshot>(emptyMemory());
  status = $state<SimStatus | null>(null);
  // Reactive sets: a single SvelteSet instance mutated in place keeps power-flow
  // highlighting reactive without relying on whole-object reassignment.
  activeElements = new SvelteSet<string>();
  activeRungs = new SvelteSet<string>();
  selectedRungId = $state<string | null>(null);
  /** Insert target: which OR branch of the selected rung (null = main series). */
  selectedBranch = $state<number | null>(null);
  /** Insert target inside an inline parallel group (group id + branch index). */
  selectedParallel = $state<{ groupId: string; branch: number } | null>(null);
  message = $state<string>("");
  busy = $state(false);
  cycleMs = $state(20);
  view = $state<AppView>("ladder");
  symbols = $state<PlcSymbol[]>([]);
  modbus = $state<ModbusStatus>({
    enabled: false,
    running: false,
    port: 5020,
    bind: "127.0.0.1",
    write_enabled: false,
    last_error: "",
  });
  modbusMap = $state<ModbusMapSnapshot>({
    entries: [],
    identity_fallback: true,
    write_protect_mode: "strict",
  });
  logs = $state<LogEntry[]>([]);
  memoryConfig = $state<MemoryConfig>({
    inputs: 128,
    outputs: 128,
    markers: 1024,
    data16: 1024,
    data32: 0,
    internal16: 1024,
    timers: 64,
    counters: 64,
  });
  memoryLimits = $state<MemoryConfig>({
    inputs: 4096,
    outputs: 4096,
    markers: 4096,
    data16: 4096,
    data32: 2048,
    internal16: 4096,
    timers: 256,
    counters: 256,
  });
  registerPool = $state(4096);

  /** Element property dialog target */
  editingElement = $state<LadderElement | null>(null);
  editingRungId = $state<string | null>(null);
  editingOrBranch = $state<number | null>(null);
  /** Explicit flag — more reliable for Svelte 5 class reactivity */
  dialogOpen = $state(false);

  private unsubs: Array<() => void> = [];

  async init() {
    const st = await api.getStatus();
    if (st.ok && st.data) {
      this.status = st.data;
      this.cycleMs = st.data.cycle_ms;
    }

    // Always bring up the dual-pump wet-well as the working project so the
    // canvas is never left on the tiny Demo_Start_Stop after a cold start.
    // (User can still switch via toolbar "Demo".)
    await this.loadWaterTank();

    const mem = await api.getMemory(true);
    if (mem.ok && mem.data) this.memory = mem.data;

    await this.refreshSymbols();
    await this.refreshModbus();
    await this.refreshMemoryConfig();

    this.unsubs.push(
      await listenScanTick((payload) => {
        const t = payload as ScanTickEvent;
        syncSet(this.activeElements, t.active_elements ?? []);
        syncSet(this.activeRungs, t.active_rungs ?? []);
        if (this.status) {
          this.status = {
            ...this.status,
            running: t.run_state === "run",
            run_state: t.run_state,
            scan_count: t.scan_count,
            last_scan_us: t.last_scan_us,
            cycle_ms: t.cycle_ms,
            fault_code: t.fault_code,
            fault_message: t.fault_message,
          };
        }
      })
    );
    this.unsubs.push(
      await listenMemory((payload) => {
        this.memory = payload as MemorySnapshot;
      })
    );
    this.unsubs.push(
      await listenFault((payload) => {
        const t = payload as ScanTickEvent;
        this.message = `FAULT ${t.fault_code}: ${t.fault_message}`;
      })
    );
  }

  destroy() {
    for (const u of this.unsubs) u();
    this.unsubs = [];
  }

  async pushProgram() {
    this.busy = true;
    const res = await api.updateProgram(normalizeProgram(this.program));
    this.busy = false;
    if (res.ok && res.data) {
      this.message = `Program loaded · ${res.data.instruction_count} instr · ${res.data.hash.slice(0, 12)}…`;
      if (this.status) {
        this.status = {
          ...this.status,
          program_hash: res.data.hash,
          program_version: res.data.version,
        };
      }
    } else {
      this.message = res.error ?? "update_program failed";
    }
    return res;
  }

  async start() {
    await this.pushProgram();
    const res = await api.start();
    if (res.ok && res.data) {
      this.status = res.data;
      this.message = "Simulation RUN";
    } else {
      this.message = res.error ?? "start failed";
    }
  }

  async stop() {
    const res = await api.stop();
    if (res.ok && res.data) {
      this.status = res.data;
      this.activeElements.clear();
      this.activeRungs.clear();
      this.message = "Simulation STOP";
    } else {
      this.message = res.error ?? "stop failed";
    }
  }

  async setCycle(ms: number) {
    this.cycleMs = ms;
    const res = await api.setCycleMs(ms);
    if (res.ok && res.data != null) this.cycleMs = res.data;
  }

  async toggleDiscrete(addr: number) {
    const cur = this.memory.discrete_inputs[addr] ?? false;
    const res = await api.setDiscrete(addr, !cur);
    if (res.ok) {
      const next = [...this.memory.discrete_inputs];
      next[addr] = !cur;
      this.memory = { ...this.memory, discrete_inputs: next };
    }
  }

  async setHolding(addr: number, value: number) {
    const res = await api.setHolding(addr, value);
    if (res.ok) {
      const next = [...this.memory.holding_registers];
      while (next.length <= addr) next.push(0);
      next[addr] = value;
      this.memory = { ...this.memory, holding_registers: next };
    }
  }

  async toggleMemoryBit(addr: number) {
    const cur = this.memory.memory_bits[addr] ?? false;
    const res = await api.setMemoryBit(addr, !cur);
    if (res.ok) {
      const next = [...this.memory.memory_bits];
      while (next.length <= addr) next.push(false);
      next[addr] = !cur;
      this.memory = { ...this.memory, memory_bits: next };
    }
  }

  async setMemoryWord(addr: number, value: number) {
    const res = await api.setMemoryWord(addr, value);
    if (res.ok) {
      const next = [...this.memory.memory_words];
      while (next.length <= addr) next.push(0);
      next[addr] = value;
      this.memory = { ...this.memory, memory_words: next };
    }
  }

  setView(v: AppView) {
    this.view = v;
  }

  async refreshSymbols() {
    const res = await api.getSymbols();
    if (res.ok && res.data) this.symbols = res.data.symbols;
  }

  async saveSymbols(symbols: PlcSymbol[]) {
    const res = await api.setSymbols(symbols);
    if (res.ok && res.data) {
      this.symbols = res.data.symbols;
      this.message = `PLC tags saved (${this.symbols.length})`;
    } else {
      this.message = res.error ?? "save symbols failed";
    }
  }

  async refreshModbus() {
    const [st, map] = await Promise.all([api.getModbusStatus(), api.getModbusMap()]);
    if (st.ok && st.data) this.modbus = st.data;
    if (map.ok && map.data) this.modbusMap = map.data;
  }

  async refreshLogs(limit = 500, level = "trace") {
    const res = await api.getLogs(limit, level);
    if (res.ok && res.data) this.logs = res.data;
  }

  async clearLogs() {
    const res = await api.clearLogs();
    if (res.ok) this.logs = [];
  }

  async refreshMemoryConfig() {
    const res = await api.getMemoryConfig();
    if (res.ok && res.data) {
      this.memoryConfig = res.data.config;
      this.memoryLimits = res.data.limits;
      this.registerPool = res.data.register_pool;
    }
  }

  async saveMemoryConfig(config: MemoryConfig): Promise<string | null> {
    const res = await api.setMemoryConfig(config);
    if (res.ok && res.data) {
      this.memoryConfig = res.data.config;
      this.memoryLimits = res.data.limits;
      this.registerPool = res.data.register_pool;
      this.message = "Memory allocation saved";
      return null;
    }
    this.message = res.error ?? "save memory allocation failed";
    return res.error ?? "save failed";
  }

  async startModbus() {
    const res = await api.startModbus();
    if (res.ok && res.data) {
      this.modbus = res.data;
      this.message = `Modbus ON · port ${res.data.port}`;
    } else {
      this.message = res.error ?? "Modbus start failed";
      await this.refreshModbus();
    }
  }

  async stopModbus() {
    const res = await api.stopModbus();
    if (res.ok && res.data) {
      this.modbus = res.data;
      this.message = "Modbus OFF";
    }
  }

  async setModbusPort(port: number) {
    const res = await api.setModbusPort(port);
    if (res.ok && res.data) {
      this.modbus = res.data;
      this.message = `Modbus port → ${port}`;
    } else {
      this.message = res.error ?? "set port failed";
      await this.refreshModbus();
    }
  }

  async setModbusWriteEnabled(allow: boolean) {
    const res = await api.setModbusWriteEnabled(allow);
    if (res.ok && res.data) {
      this.modbus = res.data;
      this.message = allow ? "Modbus writes enabled" : "Modbus read-only";
    } else {
      this.message = res.error ?? "set Modbus write mode failed";
      await this.refreshModbus();
    }
  }

  async saveModbusMap(map: ModbusMapSnapshot): Promise<string | null> {
    const res = await api.setModbusMap(map);
    if (res.ok && res.data) {
      this.modbusMap = res.data;
      this.message = `Modbus map saved (${res.data.entries.length} entries)`;
      return null;
    }
    this.message = res.error ?? "save map failed";
    return res.error ?? "save map failed";
  }

  async loadDemo() {
    const res = await api.loadDemo();
    if (res.ok) {
      this.program = createDemoProgram();
      this.message = "Demo program loaded";
      await this.refreshProgramFromBackend();
    } else {
      this.program = createDemoProgram();
      await this.pushProgram();
    }
  }

  /**
   * Dual-pump wet-well project + process-image seeds.
   * Always switches UI to ladder and replaces Demo_Start_Stop.
   * See docs/WATER_TANK_PUMP_STATION.md
   */
  async loadWaterTank() {
    this.busy = true;
    this.view = "ladder";
    this.message = "Loading Water_Tank_Dual_Pump…";
    try {
      if (this.status?.running || this.status?.run_state === "run") {
        await this.stop();
      }
      // Fresh object so Svelte 5 always re-renders the canvas (38 networks).
      const prog = normalizeProgram(createWaterTankProgram());
      this.program = prog;
      this.selectedRungId = prog.rungs[0]?.id ?? null;
      this.selectedBranch = null;
      this.selectedParallel = null;
      this.activeElements.clear();
      this.activeRungs.clear();

      const res = await api.updateProgram(prog);
      if (!res.ok) {
        this.message = `Water tank COMPILE ERROR: ${res.error ?? "update_program failed"}`;
        this.busy = false;
        return;
      }

      await this.seedWaterTankMemory();
      const tags = createWaterTankSymbols();
      const symRes = await api.setSymbols(tags);
      if (symRes.ok && symRes.data) this.symbols = symRes.data.symbols;
      else this.symbols = tags;

      // Dedicated SCADA map (HR100–150). Enable writes so SCADA can force setpoints
      // (without this, FC06 returns ServerDeviceFailure 0x04).
      const mbMap = createWaterTankModbusMap();
      const mapErr = await this.saveModbusMap(mbMap);
      await this.setModbusWriteEnabled(true);
      if (!this.modbus.running) {
        await this.startModbus();
      }
      if (mapErr) {
        this.message = `Water tank loaded · Modbus map error: ${mapErr}`;
      } else {
        this.message = `Water_Tank_Dual_Pump · ${prog.rungs.length} nets · Modbus HR100–150 · writes ON · :${this.modbus.port} · 200/700/800 cm`;
      }
    } catch (e) {
      this.message = `Water tank load failed: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      this.busy = false;
    }
  }

  /** Seed holdings/discretes for the wet-well simulation (safe after reset). */
  async seedWaterTankMemory() {
    for (const [addr, val] of Object.entries(WATER_TANK_MEMORY_SEED.holdings)) {
      await this.setHolding(Number(addr), val);
    }
    for (const [addr, val] of Object.entries(WATER_TANK_MEMORY_SEED.discretes)) {
      const a = Number(addr);
      const r = await api.setDiscrete(a, val);
      if (r.ok) {
        const next = [...this.memory.discrete_inputs];
        while (next.length <= a) next.push(false);
        next[a] = val;
        this.memory = { ...this.memory, discrete_inputs: next };
      }
    }
  }

  async refreshProgramFromBackend() {
    const prog = await api.getProgram();
    if (prog.ok && prog.data) this.program = normalizeProgram(prog.data);
  }

  async exportJson() {
    const res = await api.exportJson();
    if (res.ok && res.data) {
      downloadText(`${this.program.name}.json`, res.data);
      this.message = "Exported program JSON";
    } else {
      downloadText(`${this.program.name}.json`, JSON.stringify(this.program, null, 2));
      this.message = "Exported program JSON (client)";
    }
  }

  async importJsonFile(file: File) {
    const text = await file.text();
    const res = await api.importJson(text);
    if (res.ok) {
      this.program = normalizeProgram(JSON.parse(text) as LadderProgram);
      this.message = "Imported program";
    } else {
      try {
        this.program = normalizeProgram(JSON.parse(text) as LadderProgram);
        await this.pushProgram();
        this.message = "Imported program (client parse)";
      } catch {
        this.message = res.error ?? "import failed";
      }
    }
  }

  async exportBytecode() {
    const res = await api.exportBytecode();
    if (res.ok && res.data) {
      downloadText(`${this.program.name}.bc.hex`, res.data);
      this.message = "Exported bytecode (hex)";
    } else {
      this.message = res.error ?? "bytecode export failed";
    }
  }

  async resetMemory() {
    const res = await api.resetMemory();
    if (res.ok && res.data) {
      this.memory = res.data;
      this.message = "Process image reset";
    } else {
      this.message = res.error ?? "reset failed";
    }
  }

  addRung() {
    const r: Rung = {
      id: uid("rung"),
      comment: "New rung",
      enabled: true,
      or_branches: [],
      elements: [
        {
          type: "contact_no",
          id: uid("c"),
          address: { area: "discrete", index: 0 },
        },
        {
          type: "coil",
          id: uid("q"),
          address: { area: "coil", index: 0 },
        },
      ],
    };
    this.program = {
      ...this.program,
      rungs: [...this.program.rungs, r],
    };
    this.selectedRungId = r.id;
  }

  removeRung(id: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.filter((r) => r.id !== id),
    };
    if (this.selectedRungId === id) {
      this.selectedRungId = null;
      this.selectedBranch = null;
      this.selectedParallel = null;
    }
  }

  addElement(rungId: string, kind: PaletteKind) {
    if (kind === "or_branch") {
      this.addOrBranch(rungId);
      return;
    }
    const el = createElement(kind);
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) => {
        if (r.id !== rungId) return r;
        const rr = ensureOr(r);
        return { ...rr, elements: insertBeforeCoils(rr.elements, el) };
      }),
    };
  }

  addOrBranch(rungId: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) => {
        if (r.id !== rungId) return r;
        const rr = ensureOr(r);
        return {
          ...rr,
          or_branches: [
            ...rr.or_branches,
            [
              {
                type: "contact_no",
                id: uid("or"),
                address: { area: "discrete", index: 0 },
              },
            ],
          ],
        };
      }),
    };
  }

  removeOrBranch(rungId: string, branchIdx: number) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) => {
        if (r.id !== rungId) return r;
        const rr = ensureOr(r);
        return {
          ...rr,
          or_branches: rr.or_branches.filter((_, i) => i !== branchIdx),
        };
      }),
    };
    if (this.selectedRungId === rungId && this.selectedBranch === branchIdx) {
      this.selectedBranch = null;
    }
  }

  addToOrBranch(rungId: string, branchIdx: number, kind: PaletteKind) {
    // Coils and OR-branch tokens never belong inside a parallel contact branch.
    if (kind === "or_branch" || isCoilKind(kind)) return;
    const el = createElement(kind);
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) => {
        if (r.id !== rungId) return r;
        const rr = ensureOr(r);
        return {
          ...rr,
          or_branches: rr.or_branches.map((b, i) =>
            i === branchIdx ? [...b, el] : b
          ),
        };
      }),
    };
  }

  removeFromOrBranch(rungId: string, branchIdx: number, elementId: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) => {
        if (r.id !== rungId) return r;
        const rr = ensureOr(r);
        return {
          ...rr,
          or_branches: rr.or_branches.map((b, i) =>
            i === branchIdx ? b.filter((e) => e.id !== elementId) : b
          ),
        };
      }),
    };
  }

  updateOrElement(rungId: string, branchIdx: number, element: LadderElement) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) => {
        if (r.id !== rungId) return r;
        const rr = ensureOr(r);
        return {
          ...rr,
          or_branches: rr.or_branches.map((b, i) =>
            i === branchIdx ? b.map((e) => (e.id === element.id ? element : e)) : b
          ),
        };
      }),
    };
  }

  removeElement(rungId: string, elementId: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId
          ? { ...ensureOr(r), elements: removeNodeById(r.elements, elementId) }
          : r
      ),
    };
  }

  /**
   * Unified ladder move (left/right/up/down) — matches canvas layout:
   *   [leading OR] → [series / inline parallel] → [coils]
   *
   * Hints: selected OR/∥ branch receives an element that enters from series.
   */
  moveInRung(rungId: string, elementId: string, dir: MoveDir) {
    // null = no branch selected → new bottom OR row; number = AND into that row
    const opts = {
      orBranchHint: this.selectedBranch,
      parBranchHint: this.selectedParallel?.branch ?? null,
      parGroupHint: this.selectedParallel?.groupId ?? null,
    };
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) => {
        if (r.id !== rungId) return r;
        const rr = ensureOr(r);
        const before = { elements: rr.elements, or_branches: rr.or_branches };
        const moved = moveInRung(rr.elements, rr.or_branches, elementId, dir, opts);
        if (!moved) return r;
        const changed =
          JSON.stringify(moved.elements) !== JSON.stringify(before.elements) ||
          JSON.stringify(moved.or_branches) !== JSON.stringify(before.or_branches);
        if (changed) {
          const from = findInRung(before.elements, before.or_branches, elementId)?.kind;
          const to = findInRung(moved.elements, moved.or_branches, elementId)?.kind;
          if (from && to && from !== to) {
            if (to === "or") {
              this.message =
                this.selectedBranch != null
                  ? `→ AND into OR branch ${this.selectedBranch}`
                  : "→ new OR branch at bottom (click a branch first for AND)";
            } else if (to === "par") {
              this.message =
                this.selectedParallel != null
                  ? `→ AND into ∥ branch ${this.selectedParallel.branch}`
                  : "→ new ∥ branch at bottom (click a branch first for AND)";
            } else if (from === "or") {
              this.message = "Exited OR → series";
            } else if (from === "par") {
              this.message = "Exited parallel ∥ → series";
            } else {
              this.message = `Moved ${dir}`;
            }
          } else {
            this.message = `Moved ${dir}`;
          }
        }
        return { ...rr, elements: moved.elements, or_branches: moved.or_branches };
      }),
    };
  }

  /** Whether a direction is available for the element (for button enable state). */
  canMoveInRung(rungId: string, elementId: string, dir: MoveDir): boolean {
    const r = this.program.rungs.find((x) => x.id === rungId);
    if (!r) return false;
    const rr = ensureOr(r);
    return canMoveInRung(rr.elements, rr.or_branches, elementId, dir, {
      orBranchHint: this.selectedBranch,
      parBranchHint: this.selectedParallel?.branch ?? null,
      parGroupHint: this.selectedParallel?.groupId ?? null,
    });
  }

  updateElement(rungId: string, element: LadderElement) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId
          ? { ...ensureOr(r), elements: updateNodeById(r.elements, element) }
          : r
      ),
    };
  }

  openElementEditor(rungId: string, element: LadderElement, orBranch: number | null = null) {
    this.editingRungId = rungId;
    this.editingOrBranch = orBranch;
    this.selectedRungId = rungId;
    // Selecting the branch while editing makes the next insert/move do AND into it
    if (orBranch != null) {
      this.selectedBranch = orBranch;
      this.selectedParallel = null;
    } else {
      const r = this.program.rungs.find((x) => x.id === rungId);
      if (r) {
        const loc = findInRung(r.elements ?? [], r.or_branches ?? [], element.id);
        if (loc?.kind === "or") {
          this.selectedBranch = loc.branch;
          this.selectedParallel = null;
        } else if (loc?.kind === "par") {
          this.selectedBranch = null;
          this.selectedParallel = { groupId: loc.groupId, branch: loc.branch };
        }
      }
    }
    try {
      this.editingElement = JSON.parse(JSON.stringify(element)) as LadderElement;
    } catch {
      this.editingElement = { ...element } as LadderElement;
    }
    this.dialogOpen = true;
    this.message = `Editing: ${element.type}`;
  }

  closeElementEditor() {
    this.dialogOpen = false;
    this.editingElement = null;
    this.editingRungId = null;
    this.editingOrBranch = null;
  }

  applyElementEdit(element: LadderElement, label = "") {
    const rungId = this.editingRungId;
    if (!rungId) return;
    if (this.editingOrBranch != null) {
      this.updateOrElement(rungId, this.editingOrBranch, element);
    } else {
      this.updateElement(rungId, element);
    }
    this.setElementLabel(element.id, label);
    this.closeElementEditor();
    void this.pushProgram();
  }

  /** Per-element override label from program metadata (max 10). */
  labelFor(id: string): string {
    return this.program.metadata?.[`lbl:${id}`] ?? "";
  }

  /**
   * Label shown above a ladder element (max 10 chars):
   * 1) manual override in program metadata (`lbl:<id>`)
   * 2) PLC tag `name` for the element's primary address
   */
  labelForElement(el: {
    id: string;
    type: string;
    address?: Address;
    done_address?: Address | null;
    source?: Address;
    dest?: Address;
    a?: Address;
  }): string {
    const override = this.labelFor(el.id).trim();
    if (override) return override.slice(0, 10);
    const addr = this.resolveLabelAddress(el);
    if (!addr) return "";
    const sym = this.symbols.find((s) => s.area === addr.area && s.index === addr.index);
    return (sym?.name ?? "").trim().slice(0, 10);
  }

  private resolveLabelAddress(el: {
    type: string;
    address?: Address;
    done_address?: Address | null;
    source?: Address;
    dest?: Address;
    a?: Address;
  }): Address | null {
    if (el.address) return el.address;
    if (el.type === "move" && el.dest) return el.dest;
    if ((el.type === "math" || el.type === "compare") && el.a) return el.a;
    if (el.done_address) return el.done_address;
    if (el.dest) return el.dest;
    return null;
  }

  setElementLabel(id: string, label: string) {
    const trimmed = label.trim().slice(0, 10);
    const metadata = { ...(this.program.metadata ?? {}) };
    if (trimmed) metadata[`lbl:${id}`] = trimmed;
    else delete metadata[`lbl:${id}`];
    this.program = { ...this.program, metadata };
  }

  updateRungComment(rungId: string, comment: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId ? { ...ensureOr(r), comment } : r
      ),
    };
  }

  // ── Insertion target + toolbar-driven editing ──────────────────────────────

  /** Select a network as the insertion target (main series). */
  selectRung(rungId: string) {
    this.selectedRungId = rungId;
    this.selectedBranch = null;
    this.selectedParallel = null;
  }

  /** Select a specific parallel OR branch of a network as the insertion target. */
  selectBranch(rungId: string, branchIdx: number) {
    this.selectedRungId = rungId;
    this.selectedBranch = branchIdx;
    this.selectedParallel = null;
  }

  /** Select a branch of an inline parallel group as the insertion target. */
  selectParallelBranch(rungId: string, groupId: string, branch: number) {
    this.selectedRungId = rungId;
    this.selectedBranch = null;
    this.selectedParallel = { groupId, branch };
  }

  /** Currently selected network index, or -1. */
  get selectedRungIndex(): number {
    return this.program.rungs.findIndex((r) => r.id === this.selectedRungId);
  }

  /** Human-readable description of where the toolbar will insert. */
  get insertTargetLabel(): string {
    const i = this.selectedRungIndex;
    if (i < 0) return "new network";
    if (this.selectedParallel)
      return `Network ${i} · ∥${this.selectedParallel.branch} (AND)`;
    return this.selectedBranch != null
      ? `Network ${i} · OR${this.selectedBranch} (AND)`
      : `Network ${i} · series`;
  }

  /** Return the target rung id, creating + selecting a network when none exists. */
  private ensureTargetRung(): string {
    let id =
      this.selectedRungId ??
      this.program.rungs[this.program.rungs.length - 1]?.id ??
      null;
    if (!id) {
      this.addRung();
      id = this.program.rungs[this.program.rungs.length - 1]?.id ?? null;
    }
    if (id) this.selectedRungId = id;
    return id ?? "";
  }

  /** Insert an instruction (or a parallel branch) at the current target. */
  insertInstruction(kind: PaletteKind) {
    if (this.view !== "ladder") this.setView("ladder");
    const rungId = this.ensureTargetRung();
    if (!rungId) return;

    if (kind === "or_branch") {
      this.addOrBranch(rungId);
      const r = this.program.rungs.find((x) => x.id === rungId);
      this.selectedBranch = r ? r.or_branches.length - 1 : null;
      this.selectedParallel = null;
      this.message = "Added parallel OR branch — pick the next contact";
      return;
    }

    if (isCoilKind(kind)) {
      // Coils are outputs — always to the series output rail.
      this.addElement(rungId, kind);
      this.message = "Coil added on the output rail";
      return;
    }

    if (this.selectedParallel) {
      const { groupId, branch } = this.selectedParallel;
      const el = createElement(kind);
      this.program = {
        ...this.program,
        rungs: this.program.rungs.map((r) =>
          r.id === rungId
            ? { ...ensureOr(r), elements: addToParallelBranch(r.elements, groupId, branch, el) }
            : r
        ),
      };
      this.message = `Inserted ${kind} into parallel branch ∥${branch}`;
      return;
    }

    if (this.selectedBranch != null) {
      this.addToOrBranch(rungId, this.selectedBranch, kind);
      this.message = `Inserted ${kind} into OR${this.selectedBranch}`;
    } else {
      this.addElement(rungId, kind);
      this.message = `Inserted ${kind}`;
    }
  }

  /** Insert an inline parallel block (two seed branches) into the series. */
  addParallelBlock() {
    if (this.view !== "ladder") this.setView("ladder");
    const rungId = this.ensureTargetRung();
    if (!rungId) return;
    const group = makeParallelGroup(uid("par"), [
      [createElement("contact_no") as LadderElement],
      [createElement("contact_no") as LadderElement],
    ]);
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId ? { ...ensureOr(r), elements: insertBeforeCoils(r.elements, group) } : r
      ),
    };
    this.selectedBranch = null;
    this.selectedParallel = { groupId: group.id, branch: 0 };
    this.message = "Inserted parallel block — select a branch and add contacts";
  }

  /** Add a new branch to an inline parallel group. */
  addBranchToParallel(rungId: string, groupId: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId
          ? {
              ...ensureOr(r),
              elements: addParallelBranch(
                r.elements,
                groupId,
                createElement("contact_no") as LadderElement
              ),
            }
          : r
      ),
    };
    const r = this.program.rungs.find((x) => x.id === rungId);
    const grp = r?.elements.find((n) => n.type === "parallel" && n.id === groupId);
    this.selectedParallel = {
      groupId,
      branch: grp && grp.type === "parallel" ? grp.branches.length - 1 : 0,
    };
  }

  /** Remove one branch of an inline parallel group (drops the group if emptied). */
  removeParallelBranch(rungId: string, groupId: string, branch: number) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId
          ? { ...ensureOr(r), elements: removeParallelBranchOp(r.elements, groupId, branch) }
          : r
      ),
    };
    if (this.selectedParallel?.groupId === groupId) this.selectedParallel = null;
  }

  /** Remove an entire inline parallel group. */
  removeParallelGroup(rungId: string, groupId: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId ? { ...ensureOr(r), elements: removeNodeById(r.elements, groupId) } : r
      ),
    };
    if (this.selectedParallel?.groupId === groupId) this.selectedParallel = null;
  }

  /** Delete the selected network (if any). */
  deleteSelectedNetwork() {
    const id = this.selectedRungId;
    if (!id) return;
    this.removeRung(id);
    this.selectedBranch = null;
    this.selectedParallel = null;
  }

  isActive(id: string) {
    return this.activeElements.has(id);
  }

  isRungActive(id: string) {
    return this.activeRungs.has(id);
  }
}

function normalizeProgram(p: LadderProgram): LadderProgram {
  return {
    ...p,
    rungs: (p.rungs ?? []).map((r) => ({
      ...r,
      or_branches: r.or_branches ?? [],
      elements: r.elements ?? [],
      enabled: r.enabled !== false,
    })),
    metadata: p.metadata ?? {},
  };
}

const COIL_TYPES = new Set(["coil", "coil_negated", "coil_set", "coil_reset"]);

function isCoilKind(kind: string): boolean {
  return COIL_TYPES.has(kind);
}

function downloadText(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export const plc = new PlcStore();
