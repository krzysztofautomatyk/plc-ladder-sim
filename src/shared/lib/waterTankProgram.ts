/**
 * Dual-pump wet-well LAD with hydrostatic probe, inflow factor K,
 * fault failover, locks, run-time and start counters.
 *
 * Level: R100 in cm (0…1000). Hysteresis: ON≥700, P2≥800, OFF≤200.
 * K×100: 50 = 0.50, 150 = 1.50  (inflow / one-pump capacity)
 *
 * Full register map: docs/WATER_TANK_PUMP_STATION.md
 */
import type {
  Address,
  LadderElement,
  LadderProgram,
  PlcSymbol,
  Rung,
  RungNode,
} from "./types";

let _seq = 0;
function id(p: string): string {
  _seq += 1;
  return `${p}_${_seq}`;
}

const I = (n: number): Address => ({ area: "discrete", index: n });
const Q = (n: number): Address => ({ area: "coil", index: n });
const M = (n: number): Address => ({ area: "memory_bit", index: n });
const R = (n: number): Address => ({ area: "holding", index: n });
/** Timer value bank TV n */
const TV = (n: number): Address => ({ area: "holding", index: 2048 + 2 * n });
/** Counter value bank CV n */
const CV = (n: number): Address => ({ area: "holding", index: 3072 + 2 * n });

function rung(comment: string, elements: RungNode[], or: LadderElement[][] = []): Rung {
  return { id: id("rung"), comment, enabled: true, or_branches: or, elements };
}
function no(a: Address): LadderElement {
  return { type: "contact_no", id: id("c"), address: a };
}
function nc(a: Address): LadderElement {
  return { type: "contact_nc", id: id("c"), address: a };
}
function coil(a: Address): LadderElement {
  return { type: "coil", id: id("coil"), address: a };
}
function ton(idx: number, ms: number, done: Address): LadderElement {
  return { type: "ton", id: id("ton"), preset_ms: ms, timer_index: idx, done_address: done };
}
function rto(idx: number, ms: number, reset: Address | null): LadderElement {
  return {
    type: "rto",
    id: id("rto"),
    preset_ms: ms,
    timer_index: idx,
    done_address: null,
    reset_address: reset,
  };
}
function ctu(idx: number, reset: Address | null): LadderElement {
  return {
    type: "ctu",
    id: id("ctu"),
    preset: 65535,
    counter_index: idx,
    done_address: null,
    reset_address: reset,
  };
}
function mov(src: Address, dest: Address): LadderElement {
  return { type: "move", id: id("mov"), source: src, dest };
}
function cmp(
  op: "eq" | "ne" | "gt" | "ge" | "lt" | "le",
  a: Address,
  b: Address
): LadderElement {
  return { type: "compare", id: id("cmp"), op, a, b };
}
function math(
  op: "add" | "sub" | "mul" | "div",
  a: Address,
  b: Address,
  dest: Address
): LadderElement {
  return { type: "math", id: id("math"), op, a, b, dest };
}
function rising(a: Address): LadderElement {
  return { type: "contact_rising", id: id("re"), address: a };
}
function set(a: Address): LadderElement {
  return { type: "coil_set", id: id("set"), address: a };
}
function rst(a: Address): LadderElement {
  return { type: "coil_reset", id: id("rst"), address: a };
}
function par(branches: LadderElement[][]): RungNode {
  return { type: "parallel", id: id("par"), branches };
}

export function createWaterTankProgram(): LadderProgram {
  _seq = 0;
  const rungs: Rung[] = [];

  // ── SIM CLOCK ─────────────────────────────────────────────────────────
  rungs.push(
    rung("[SIM] 200 ms free-run tick M0 (TON T15, NC self-reset)", [
      nc(M(0)),
      ton(15, 200, M(0)),
    ])
  );

  // ── PUMP_STEP = FILL × 100 / K ────────────────────────────────────────
  rungs.push(
    rung("[SIM] PUMP_STEP = FILL×100/K when K>0 (R103)", [
      cmp("gt", R(101), R(109)),
      math("mul", R(102), R(108), R(111)),
      math("div", R(111), R(101), R(103)),
    ])
  );

  // ── Pump count → capacity ─────────────────────────────────────────────
  rungs.push(
    rung("[SIM] ANY pump M22 = Q0 ∨ Q1", [coil(M(22))], [[no(Q(0))], [no(Q(1))]])
  );
  rungs.push(rung("[SIM] BOTH pumps M21 = Q0 ∧ Q1", [no(Q(0)), no(Q(1)), coil(M(21))]));
  rungs.push(
    rung("[SIM] ONE pump M20 = ANY ∧ ¬BOTH", [no(M(22)), nc(M(21)), coil(M(20))])
  );
  rungs.push(rung("[SIM] CAP ← PUMP_STEP if ONE", [no(M(20)), mov(R(103), R(112))]));
  rungs.push(
    rung("[SIM] CAP ← 2×PUMP_STEP if BOTH", [
      no(M(21)),
      math("add", R(103), R(103), R(112)),
    ])
  );
  rungs.push(rung("[SIM] CAP ← 0 if idle", [nc(M(22)), mov(R(109), R(112))]));

  // ── Regime: drain vs fill ─────────────────────────────────────────────
  rungs.push(
    rung("[SIM] Drain regime M25 when CAP ≥ FILL", [
      cmp("ge", R(112), R(102)),
      coil(M(25)),
    ])
  );
  rungs.push(
    rung("[SIM] DRAIN = CAP − FILL", [no(M(25)), math("sub", R(112), R(102), R(113))])
  );
  rungs.push(
    rung("[SIM] FILL_NET = FILL − CAP", [
      nc(M(25)),
      math("sub", R(102), R(112), R(114)),
    ])
  );

  // ── Integrate level on tick + SIM_EN (I0) ─────────────────────────────
  rungs.push(
    rung("[SIM] Tick∧EN∧drain: LEVEL ≥ DRAIN → LEVEL −= DRAIN", [
      no(I(0)),
      no(M(0)),
      no(M(25)),
      cmp("ge", R(100), R(113)),
      math("sub", R(100), R(113), R(100)),
    ])
  );
  rungs.push(
    rung("[SIM] Tick∧EN∧drain: LEVEL < DRAIN → LEVEL = 0", [
      no(I(0)),
      no(M(0)),
      no(M(25)),
      cmp("lt", R(100), R(113)),
      mov(R(109), R(100)),
    ])
  );
  rungs.push(
    rung("[SIM] Tick∧EN∧fill: TMP = LEVEL + FILL_NET", [
      no(I(0)),
      no(M(0)),
      nc(M(25)),
      math("add", R(100), R(114), R(104)),
    ])
  );
  rungs.push(
    rung("[SIM] Tick∧EN∧fill: TMP ≤ 1000 → LEVEL = TMP", [
      no(I(0)),
      no(M(0)),
      nc(M(25)),
      cmp("le", R(104), R(110)),
      mov(R(104), R(100)),
    ])
  );
  rungs.push(
    rung("[SIM] Tick∧EN∧fill: TMP > 1000 → LEVEL = 1000", [
      no(I(0)),
      no(M(0)),
      nc(M(25)),
      cmp("gt", R(104), R(110)),
      mov(R(110), R(100)),
    ])
  );

  // ── Sensors (LEVEL in cm: R100) ───────────────────────────────────────
  // R105=SP_STOP(200), R106=SP_P1_ON(700), R107=SP_P2_ON(800)
  rungs.push(
    rung("[SENS] Above stop band M10 = LEVEL > SP_STOP (200 cm)", [
      cmp("gt", R(100), R(105)),
      coil(M(10)),
    ])
  );
  rungs.push(
    rung("[SENS] Start-band / HI M11 = LEVEL ≥ SP_P1_ON (700 cm)", [
      cmp("ge", R(100), R(106)),
      coil(M(11)),
    ])
  );
  rungs.push(
    rung(
      "[SENS] ALM_HI Q2 = LEVEL≥700 ∨ hardwired float HI I2",
      [coil(Q(2))],
      [[no(M(11))], [no(I(2))]]
    )
  );

  // ── Hysteresis (latches — no chattering on the falling edge) ─────────
  // DEMAND M2:  SET @ ≥700 cm   RESET @ ≤200 cm
  // JOIN_P2 M3: SET @ ≥800 cm   RESET @ ≤200 cm (same stop) — stays ON while emptying
  rungs.push(
    rung("[CTRL] DEMAND SET  LEVEL ≥ 700 cm (SP_P1_ON)", [
      cmp("ge", R(100), R(106)),
      set(M(2)),
    ])
  );
  rungs.push(
    rung("[CTRL] DEMAND RESET  LEVEL ≤ 200 cm (SP_STOP) — all pumps off", [
      cmp("le", R(100), R(105)),
      rst(M(2)),
    ])
  );
  rungs.push(
    rung("[CTRL] JOIN_P2 SET  LEVEL ≥ 800 cm (SP_P2_ON) — latch 2nd pump", [
      cmp("ge", R(100), R(107)),
      set(M(3)),
    ])
  );
  rungs.push(
    rung("[CTRL] JOIN_P2 RESET  LEVEL ≤ 200 cm — release with DEMAND", [
      cmp("le", R(100), R(105)),
      rst(M(3)),
    ])
  );
  // Safety: if DEMAND is gone, clear JOIN_P2 (e.g. manual edge cases)
  rungs.push(
    rung("[CTRL] JOIN_P2 RESET when DEMAND cleared", [nc(M(2)), rst(M(3))])
  );

  // ── Availability ──────────────────────────────────────────────────────
  rungs.push(
    rung("[P1] OK M4 = ¬FAULT(I3) ∧ ¬LOCK(I5)", [nc(I(3)), nc(I(5)), coil(M(4))])
  );
  rungs.push(
    rung("[P2] OK M5 = ¬FAULT(I4) ∧ ¬LOCK(I6)", [nc(I(4)), nc(I(6)), coil(M(5))])
  );
  rungs.push(
    rung("[LEAD] M7 = P2 is lead when P1 not OK", [nc(M(4)), coil(M(7))])
  );

  // ── Commands: both hold until SP_STOP (200 cm), no level-band chatter ─
  rungs.push(
    rung("[P1] CMD M6 = P1_OK ∧ (DEMAND ∨ MAN I8)", [
      no(M(4)),
      par([[no(M(2))], [no(I(8))]]),
      coil(M(6)),
    ])
  );
  // P2: once JOIN_P2 latched at 800 cm, stays until 200 cm (M3), OR failover
  rungs.push(
    rung(
      "[P2] CMD M8 = P2_OK ∧ DEMAND ∧ (JOIN_P2 latch ∨ P1 down ∨ P1 FAULT ∨ MAN I9)",
      [
        no(M(5)),
        no(M(2)),
        par([[no(M(3))], [nc(M(4))], [no(I(3))], [no(I(9))], [no(M(7))]]),
        coil(M(8)),
      ]
    )
  );
  rungs.push(rung("[OUT] Pump 1 contactor Q0 ← M6", [no(M(6)), coil(Q(0))]));
  rungs.push(rung("[OUT] Pump 2 contactor Q1 ← M8", [no(M(8)), coil(Q(1))]));

  // ── Alarms ────────────────────────────────────────────────────────────
  rungs.push(
    rung(
      "[ALM] Fault lamp Q3 = P1_FAULT ∨ P2_FAULT",
      [coil(Q(3))],
      [[no(I(3))], [no(I(4))]]
    )
  );
  rungs.push(
    rung("[ALM] Station fail Q4 = DEMAND ∧ ¬P1_OK ∧ ¬P2_OK", [
      no(M(2)),
      nc(M(4)),
      nc(M(5)),
      coil(Q(4)),
    ])
  );

  // ── Statistics: run time HH:MM:SS (3 regs each) + start counts ────────
  // R115=1, R116=60 constants · R117/R118 scratch
  // P1: R120 HH, R121 MM, R122 SS · P2: R123 HH, R124 MM, R125 SS
  // Starts: R126 P1, R127 P2
  rungs.push(
    rung("[STAT] 1 s free-run tick M40 (TON T14) for run-time clocks", [
      nc(M(40)),
      ton(14, 1000, M(40)),
    ])
  );

  // P1 SS++
  rungs.push(
    rung("[STAT] P1: on 1s∧Q0 → TMP=SS+1", [
      no(Q(0)),
      no(M(40)),
      math("add", R(122), R(115), R(117)),
    ])
  );
  rungs.push(
    rung("[STAT] P1: SS+1 < 60 → SS=TMP", [
      no(Q(0)),
      no(M(40)),
      cmp("lt", R(117), R(116)),
      mov(R(117), R(122)),
    ])
  );
  rungs.push(
    rung("[STAT] P1: SS+1 ≥ 60 → SS=0, TMP2=MM+1", [
      no(Q(0)),
      no(M(40)),
      cmp("ge", R(117), R(116)),
      mov(R(109), R(122)),
      math("add", R(121), R(115), R(118)),
    ])
  );
  rungs.push(
    rung("[STAT] P1: MM+1 < 60 → MM=TMP2", [
      no(Q(0)),
      no(M(40)),
      cmp("ge", R(117), R(116)),
      cmp("lt", R(118), R(116)),
      mov(R(118), R(121)),
    ])
  );
  rungs.push(
    rung("[STAT] P1: MM+1 ≥ 60 → MM=0, HH=HH+1", [
      no(Q(0)),
      no(M(40)),
      cmp("ge", R(117), R(116)),
      cmp("ge", R(118), R(116)),
      mov(R(109), R(121)),
      math("add", R(120), R(115), R(120)),
    ])
  );

  // P2 SS++
  rungs.push(
    rung("[STAT] P2: on 1s∧Q1 → TMP=SS+1", [
      no(Q(1)),
      no(M(40)),
      math("add", R(125), R(115), R(117)),
    ])
  );
  rungs.push(
    rung("[STAT] P2: SS+1 < 60 → SS=TMP", [
      no(Q(1)),
      no(M(40)),
      cmp("lt", R(117), R(116)),
      mov(R(117), R(125)),
    ])
  );
  rungs.push(
    rung("[STAT] P2: SS+1 ≥ 60 → SS=0, TMP2=MM+1", [
      no(Q(1)),
      no(M(40)),
      cmp("ge", R(117), R(116)),
      mov(R(109), R(125)),
      math("add", R(124), R(115), R(118)),
    ])
  );
  rungs.push(
    rung("[STAT] P2: MM+1 < 60 → MM=TMP2", [
      no(Q(1)),
      no(M(40)),
      cmp("ge", R(117), R(116)),
      cmp("lt", R(118), R(116)),
      mov(R(118), R(124)),
    ])
  );
  rungs.push(
    rung("[STAT] P2: MM+1 ≥ 60 → MM=0, HH=HH+1", [
      no(Q(1)),
      no(M(40)),
      cmp("ge", R(117), R(116)),
      cmp("ge", R(118), R(116)),
      mov(R(109), R(124)),
      math("add", R(123), R(115), R(123)),
    ])
  );

  // Reset stats on I7
  rungs.push(
    rung("[STAT] I7: clear P1 HH/MM/SS", [
      no(I(7)),
      mov(R(109), R(120)),
      mov(R(109), R(121)),
      mov(R(109), R(122)),
    ])
  );
  rungs.push(
    rung("[STAT] I7: clear P2 HH/MM/SS", [
      no(I(7)),
      mov(R(109), R(123)),
      mov(R(109), R(124)),
      mov(R(109), R(125)),
    ])
  );

  rungs.push(
    rung("[STAT] P1 starts CTU C0 on ↑M6; reset I7", [rising(M(6)), ctu(0, I(7))])
  );
  rungs.push(rung("[STAT] R126 ← CV0 (P1 starts)", [mov(CV(0), R(126))]));
  rungs.push(
    rung("[STAT] P2 starts CTU C1 on ↑M8; reset I7", [rising(M(8)), ctu(1, I(7))])
  );
  rungs.push(rung("[STAT] R127 ← CV1 (P2 starts)", [mov(CV(1), R(127))]));

  return {
    name: "Water_Tank_Dual_Pump",
    version: "1.0.0",
    description:
      "Dual-pump wet-well with hydrostatic SP, inflow factor K, fault failover, locks, run-time & starts. Enable sim on I0.",
    rungs,
    metadata: {
      author: "process engineering pack",
      standard: "IEC 61131-3 LAD subset",
      doc: "docs/WATER_TANK_PUMP_STATION.md",
      k_scale: "R101 = K*100 (50=0.5, 150=1.5)",
      level_scale: "R100 = cm (0..1000)",
      hysteresis: "P1 ON≥700cm, P2 ON≥800cm, both OFF≤200cm",
    },
  };
}

/** Process-image seeds applied when loading this project into the simulator. */
export const WATER_TANK_MEMORY_SEED = {
  holdings: {
    100: 500, // LEVEL cm (mid tank — will fill to 700 then pump)
    101: 150, // K = 1.50
    102: 10, // FILL_STEP cm / tick
    105: 200, // SP_STOP 200 cm — pumps off
    106: 700, // SP_P1_ON 700 cm — start pump 1
    107: 800, // SP_P2_ON 800 cm — add pump 2
    108: 100, // CONST 100
    109: 0, // CONST 0
    110: 1000, // CONST 1000 (max cm)
    115: 1, // CONST 1 (time clocks)
    116: 60, // CONST 60 (SS/MM rollover)
    120: 0, // P1_HH
    121: 0, // P1_MM
    122: 0, // P1_SS
    123: 0, // P2_HH
    124: 0, // P2_MM
    125: 0, // P2_SS
    126: 0, // P1_STARTS
    127: 0, // P2_STARTS
  } as Record<number, number>,
  discretes: {
    0: true, // SIM_EN
  } as Record<number, boolean>,
};

/** PLC tags: name = max 10 char label, comment = full description. */
export function createWaterTankSymbols(): PlcSymbol[] {
  const bit = (
    id: string,
    name: string,
    area: PlcSymbol["area"],
    index: number,
    comment: string,
    addr: string
  ): PlcSymbol => ({
    id,
    name: name.slice(0, 10),
    area,
    index,
    data_type: "bool",
    comment,
    address_display: addr,
  });
  const word = (
    id: string,
    name: string,
    index: number,
    comment: string
  ): PlcSymbol => ({
    id,
    name: name.slice(0, 10),
    area: "holding",
    index,
    data_type: "word",
    comment,
    address_display: `R${index}`,
  });

  return [
    bit("wt_i0", "SIM_EN", "discrete", 0, "Enable level simulation (inflow/outflow model)", "I0"),
    bit("wt_i1", "FLT_LO", "discrete", 1, "Hardwired low float (backup for stop band)", "I1"),
    bit("wt_i2", "FLT_HI", "discrete", 2, "Hardwired high float (backup high alarm)", "I2"),
    bit("wt_i3", "P1_FAULT", "discrete", 3, "Pump 1 fault / trip — triggers failover to P2", "I3"),
    bit("wt_i4", "P2_FAULT", "discrete", 4, "Pump 2 fault / trip", "I4"),
    bit("wt_i5", "P1_LOCK", "discrete", 5, "Operator lock — blocks pump 1", "I5"),
    bit("wt_i6", "P2_LOCK", "discrete", 6, "Operator lock — blocks pump 2", "I6"),
    bit("wt_i7", "RST_STAT", "discrete", 7, "Reset run-time RTO and start counters", "I7"),
    bit("wt_i8", "MAN_P1", "discrete", 8, "Manual force pump 1 (if available)", "I8"),
    bit("wt_i9", "MAN_P2", "discrete", 9, "Manual force pump 2 (if available)", "I9"),

    bit("wt_q0", "P1_RUN", "coil", 0, "Pump 1 contactor / run command", "Q0"),
    bit("wt_q1", "P2_RUN", "coil", 1, "Pump 2 contactor / run command", "Q1"),
    bit("wt_q2", "ALM_HI", "coil", 2, "High level alarm (≥700 cm or float HI)", "Q2"),
    bit("wt_q3", "ALM_FLT", "coil", 3, "Any pump fault lamp", "Q3"),
    bit("wt_q4", "ALM_FAIL", "coil", 4, "Station fail: demand but no pump available", "Q4"),

    word("wt_r100", "LEVEL_cm", 100, "Hydrostatic level [cm], 0…1000"),
    word("wt_r101", "K_x100", 101, "Inflow factor ×100 (50=0.5, 150=1.5). One pump capacity vs inflow."),
    word("wt_r102", "FILL_STEP", 102, "Sim: cm rise per 200 ms tick when no net pump capacity"),
    word("wt_r103", "PUMP_STEP", 103, "Sim: single-pump capacity cm/tick = FILL×100/K"),
    word("wt_r105", "SP_STOP", 105, "Stop level [cm] — DEMAND reset, all pumps off (default 200)"),
    word("wt_r106", "SP_P1_ON", 106, "Start level [cm] — DEMAND set, pump 1 ON (default 700)"),
    word("wt_r107", "SP_P2_ON", 107, "Assist level [cm] — pump 2 joins (default 800)"),
    word("wt_r108", "C100", 108, "Constant 100 for K math"),
    word("wt_r109", "C0", 109, "Constant 0"),
    word("wt_r110", "C1000", 110, "Constant 1000 (max level clamp)"),
    word("wt_r120", "P1_HH", 120, "Pump 1 run time — hours"),
    word("wt_r121", "P1_MM", 121, "Pump 1 run time — minutes (0–59)"),
    word("wt_r122", "P1_SS", 122, "Pump 1 run time — seconds (0–59)"),
    word("wt_r123", "P2_HH", 123, "Pump 2 run time — hours"),
    word("wt_r124", "P2_MM", 124, "Pump 2 run time — minutes (0–59)"),
    word("wt_r125", "P2_SS", 125, "Pump 2 run time — seconds (0–59)"),
    word("wt_r126", "P1_STARTS", 126, "Pump 1 start counter"),
    word("wt_r127", "P2_STARTS", 127, "Pump 2 start counter"),
    word("wt_r115", "C1", 115, "Constant 1 (clock increment)"),
    word("wt_r116", "C60", 116, "Constant 60 (SS/MM rollover)"),

    bit("wt_m2", "DEMAND", "memory_bit", 2, "Pump-out demand latch: SET≥700 cm, RESET≤200 cm", "M2"),
    bit("wt_m3", "JOIN_P2", "memory_bit", 3, "2nd pump latch: SET≥800 cm, RESET≤200 cm (no chatter)", "M3"),
    bit("wt_m4", "P1_OK", "memory_bit", 4, "Pump 1 available (¬fault ∧ ¬lock)", "M4"),
    bit("wt_m5", "P2_OK", "memory_bit", 5, "Pump 2 available (¬fault ∧ ¬lock)", "M5"),
    bit("wt_m6", "P1_CMD", "memory_bit", 6, "Internal command for pump 1", "M6"),
    bit("wt_m8", "P2_CMD", "memory_bit", 8, "Internal command for pump 2", "M8"),
  ];
}
