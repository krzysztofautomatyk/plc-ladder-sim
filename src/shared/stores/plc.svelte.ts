/**
 * Reactive PLC application store (Svelte 5 runes).
 */
import { api, listenFault, listenMemory, listenScanTick } from "../lib/api";
import { createDemoProgram, uid } from "../lib/demoProgram";
import { createElement } from "../../features/ladder/elements";
import { SvelteSet } from "svelte/reactivity";
import type {
  AppView,
  LadderElement,
  LadderProgram,
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
  return {
    coils: Array(64).fill(false),
    discrete_inputs: Array(64).fill(false),
    holding_registers: Array(64).fill(0),
    input_registers: Array(16).fill(0),
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
  program = $state<LadderProgram>(createDemoProgram());
  memory = $state<MemorySnapshot>(emptyMemory());
  status = $state<SimStatus | null>(null);
  // Reactive sets: a single SvelteSet instance mutated in place keeps power-flow
  // highlighting reactive without relying on whole-object reassignment.
  activeElements = new SvelteSet<string>();
  activeRungs = new SvelteSet<string>();
  selectedRungId = $state<string | null>(null);
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
  modbusMap = $state<ModbusMapSnapshot>({ entries: [], identity_fallback: true });

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
    const prog = await api.getProgram();
    if (prog.ok && prog.data) {
      this.program = normalizeProgram(prog.data);
    } else {
      await this.pushProgram();
    }
    const mem = await api.getMemory(true);
    if (mem.ok && mem.data) this.memory = mem.data;

    await this.refreshSymbols();
    await this.refreshModbus();

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

  async saveModbusMap(map: ModbusMapSnapshot) {
    const res = await api.setModbusMap(map);
    if (res.ok && res.data) {
      this.modbusMap = res.data;
      this.message = `Modbus map saved (${res.data.entries.length} entries)`;
    } else {
      this.message = res.error ?? "save map failed";
    }
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
    if (this.selectedRungId === id) this.selectedRungId = null;
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
        return { ...rr, elements: insertBeforeCoil(rr.elements, el) };
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
  }

  addToOrBranch(rungId: string, branchIdx: number, kind: PaletteKind) {
    if (kind === "or_branch") return;
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
          ? { ...ensureOr(r), elements: r.elements.filter((e) => e.id !== elementId) }
          : r
      ),
    };
  }

  updateElement(rungId: string, element: LadderElement) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId
          ? {
              ...ensureOr(r),
              elements: r.elements.map((e) => (e.id === element.id ? element : e)),
            }
          : r
      ),
    };
  }

  openElementEditor(rungId: string, element: LadderElement, orBranch: number | null = null) {
    this.editingRungId = rungId;
    this.editingOrBranch = orBranch;
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

  applyElementEdit(element: LadderElement) {
    const rungId = this.editingRungId;
    if (!rungId) return;
    if (this.editingOrBranch != null) {
      this.updateOrElement(rungId, this.editingOrBranch, element);
    } else {
      this.updateElement(rungId, element);
    }
    this.closeElementEditor();
    void this.pushProgram();
  }

  updateRungComment(rungId: string, comment: string) {
    this.program = {
      ...this.program,
      rungs: this.program.rungs.map((r) =>
        r.id === rungId ? { ...ensureOr(r), comment } : r
      ),
    };
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

function insertBeforeCoil(elements: LadderElement[], el: LadderElement): LadderElement[] {
  const coilIdx = elements.findIndex(
    (e) => e.type === "coil" || e.type === "coil_negated"
  );
  if (coilIdx === -1) return [...elements, el];
  if (el.type === "coil" || el.type === "coil_negated") return [...elements, el];
  const next = [...elements];
  next.splice(coilIdx, 0, el);
  return next;
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
