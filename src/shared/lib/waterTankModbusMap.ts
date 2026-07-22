/**
 * Compact Modbus map for Water_Tank_Dual_Pump — **all process data in HR 100…150**.
 *
 * | HR  | Content                                              |
 * |-----|------------------------------------------------------|
 * | 100 | I0–I15 packed (bit0=I0 … bit9=I9)                    |
 * | 101 | Q0–Q15 packed (bit0=Q0 … bit4=Q4)                    |
 * | 102 | M0–M15 packed (DEMAND, JOIN_P2, P1_OK, …)            |
 * | 103 | M16–M31 packed (ONE_P, BOTH, ANY, DRAIN, …)          |
 * | 104…121 | Process / setpoints / stats (→ PLC R100…)         |
 * | 122…150 | Reserved for expansion                             |
 *
 * Docs: docs/WATER_TANK_MODBUS_MAP.md
 */
import type { ModbusMapEntry, ModbusMapSnapshot } from "./types";

export type RegisterAccess = "R" | "R/W";

/** One row of the operator-facing register map table (UI + docs). */
export interface WaterTankRegisterRow {
  hr: number;
  symbol: string;
  plc: string;
  access: RegisterAccess;
  description: string;
  /** Bit legend for packed words (optional). */
  bits?: { bit: number; tag: string; meaning: string }[];
  /** How to resolve a live value for the UI table. */
  live:
    | { kind: "holding"; plcR: number }
    | { kind: "pack_i"; start: number }
    | { kind: "pack_q"; start: number }
    | { kind: "pack_m"; start: number }
    | { kind: "reserved" };
}

function direct(opts: {
  id: string;
  symbol: string;
  plc_start: number;
  modbus_start: number;
  write_protected: boolean;
  comment: string;
}): ModbusMapEntry {
  return {
    id: opts.id,
    enabled: true,
    mapping_type: "direct",
    symbol_name: opts.symbol,
    plc_area: "holding",
    plc_start: opts.plc_start,
    plc_bit_offset: 0,
    modbus_table: "holding",
    modbus_start: opts.modbus_start,
    length: 1,
    is_write_protected: opts.write_protected,
    comment: opts.comment,
  };
}

function bitToReg(opts: {
  id: string;
  symbol: string;
  plc_area: "discrete" | "coil" | "memory_bit";
  plc_start: number;
  modbus_start: number;
  comment: string;
}): ModbusMapEntry {
  return {
    id: opts.id,
    enabled: true,
    mapping_type: "bit_to_register",
    symbol_name: opts.symbol,
    plc_area: opts.plc_area,
    plc_start: opts.plc_start,
    plc_bit_offset: 0,
    modbus_table: "holding",
    modbus_start: opts.modbus_start,
    length: 1,
    is_write_protected: true,
    comment: opts.comment,
  };
}

/** Operator table HR100–150 (full bit legends + process words). */
export function waterTankRegisterTable(): WaterTankRegisterRow[] {
  const rows: WaterTankRegisterRow[] = [
    {
      hr: 100,
      symbol: "DI_PACK",
      plc: "I0…I15",
      access: "R",
      description: "Wejścia dyskretne spakowane (1 rejestr = 16 bitów)",
      bits: [
        { bit: 0, tag: "I0", meaning: "SIM_EN — włącz sim poziomu" },
        { bit: 1, tag: "I1", meaning: "FLT_LO — pływak niski" },
        { bit: 2, tag: "I2", meaning: "FLT_HI — pływak wysoki" },
        { bit: 3, tag: "I3", meaning: "P1_FAULT" },
        { bit: 4, tag: "I4", meaning: "P2_FAULT" },
        { bit: 5, tag: "I5", meaning: "P1_LOCK" },
        { bit: 6, tag: "I6", meaning: "P2_LOCK" },
        { bit: 7, tag: "I7", meaning: "RST_STAT" },
        { bit: 8, tag: "I8", meaning: "MAN_P1" },
        { bit: 9, tag: "I9", meaning: "MAN_P2" },
        { bit: 10, tag: "I10", meaning: "— rezerwa" },
        { bit: 11, tag: "I11", meaning: "— rezerwa" },
        { bit: 12, tag: "I12", meaning: "— rezerwa" },
        { bit: 13, tag: "I13", meaning: "— rezerwa" },
        { bit: 14, tag: "I14", meaning: "— rezerwa" },
        { bit: 15, tag: "I15", meaning: "— rezerwa" },
      ],
      live: { kind: "pack_i", start: 0 },
    },
    {
      hr: 101,
      symbol: "DO_PACK",
      plc: "Q0…Q15",
      access: "R",
      description: "Wyjścia (cewki) spakowane",
      bits: [
        { bit: 0, tag: "Q0", meaning: "P1_RUN" },
        { bit: 1, tag: "Q1", meaning: "P2_RUN" },
        { bit: 2, tag: "Q2", meaning: "ALM_HI" },
        { bit: 3, tag: "Q3", meaning: "ALM_FLT" },
        { bit: 4, tag: "Q4", meaning: "ALM_FAIL" },
      ],
      live: { kind: "pack_q", start: 0 },
    },
    {
      hr: 102,
      symbol: "M_LO",
      plc: "M0…M15",
      access: "R",
      description: "Markery sterowania (niższe)",
      bits: [
        { bit: 0, tag: "M0", meaning: "TICK 200 ms" },
        { bit: 2, tag: "M2", meaning: "DEMAND" },
        { bit: 3, tag: "M3", meaning: "JOIN_P2" },
        { bit: 4, tag: "M4", meaning: "P1_OK" },
        { bit: 5, tag: "M5", meaning: "P2_OK" },
        { bit: 6, tag: "M6", meaning: "P1_CMD" },
        { bit: 7, tag: "M7", meaning: "P2_LEAD" },
        { bit: 8, tag: "M8", meaning: "P2_CMD" },
        { bit: 10, tag: "M10", meaning: "V_FLT_LO" },
        { bit: 11, tag: "M11", meaning: "V_FLT_HI" },
      ],
      live: { kind: "pack_m", start: 0 },
    },
    {
      hr: 103,
      symbol: "M_HI",
      plc: "M16…M31",
      access: "R",
      description: "Markery sim / tryby pomp",
      bits: [
        { bit: 4, tag: "M20", meaning: "ONE_P — dokładnie jedna pompa" },
        { bit: 5, tag: "M21", meaning: "BOTH — obie pompy" },
        { bit: 6, tag: "M22", meaning: "ANY_P — co najmniej jedna" },
        { bit: 9, tag: "M25", meaning: "DRAIN_REGIME" },
      ],
      live: { kind: "pack_m", start: 16 },
    },
    {
      hr: 104,
      symbol: "LEVEL_cm",
      plc: "R100",
      access: "R/W",
      description: "Poziom 0…1000 cm (sim może nadpisać przy RUN+SIM_EN)",
      live: { kind: "holding", plcR: 100 },
    },
    {
      hr: 105,
      symbol: "K_x100",
      plc: "R101",
      access: "R/W",
      description: "Wsp. napływu ×100 (50=0.5, 150=1.5)",
      live: { kind: "holding", plcR: 101 },
    },
    {
      hr: 106,
      symbol: "FILL_STEP",
      plc: "R102",
      access: "R/W",
      description: "Sim: cm na tick 200 ms",
      live: { kind: "holding", plcR: 102 },
    },
    {
      hr: 107,
      symbol: "PUMP_STEP",
      plc: "R103",
      access: "R",
      description: "Wydajność 1 pompy cm/tick = FILL×100/K",
      live: { kind: "holding", plcR: 103 },
    },
    {
      hr: 108,
      symbol: "SP_STOP",
      plc: "R105",
      access: "R/W",
      description: "Stop / RESET DEMAND [cm] (domyślnie 200)",
      live: { kind: "holding", plcR: 105 },
    },
    {
      hr: 109,
      symbol: "SP_P1_ON",
      plc: "R106",
      access: "R/W",
      description: "Start P1 / SET DEMAND [cm] (domyślnie 700)",
      live: { kind: "holding", plcR: 106 },
    },
    {
      hr: 110,
      symbol: "SP_P2_ON",
      plc: "R107",
      access: "R/W",
      description: "Dołączenie P2 [cm] (domyślnie 800)",
      live: { kind: "holding", plcR: 107 },
    },
    {
      hr: 111,
      symbol: "CAP",
      plc: "R112",
      access: "R",
      description: "Całkowita wydajność pomp w ticku",
      live: { kind: "holding", plcR: 112 },
    },
    {
      hr: 112,
      symbol: "DRAIN",
      plc: "R113",
      access: "R",
      description: "CAP − FILL (tryb opróżniania)",
      live: { kind: "holding", plcR: 113 },
    },
    {
      hr: 113,
      symbol: "FILL_NET",
      plc: "R114",
      access: "R",
      description: "FILL − CAP (tryb napełniania)",
      live: { kind: "holding", plcR: 114 },
    },
    {
      hr: 114,
      symbol: "P1_HH",
      plc: "R120",
      access: "R",
      description: "Czas pracy P1 — godziny",
      live: { kind: "holding", plcR: 120 },
    },
    {
      hr: 115,
      symbol: "P1_MM",
      plc: "R121",
      access: "R",
      description: "Czas pracy P1 — minuty",
      live: { kind: "holding", plcR: 121 },
    },
    {
      hr: 116,
      symbol: "P1_SS",
      plc: "R122",
      access: "R",
      description: "Czas pracy P1 — sekundy",
      live: { kind: "holding", plcR: 122 },
    },
    {
      hr: 117,
      symbol: "P2_HH",
      plc: "R123",
      access: "R",
      description: "Czas pracy P2 — godziny",
      live: { kind: "holding", plcR: 123 },
    },
    {
      hr: 118,
      symbol: "P2_MM",
      plc: "R124",
      access: "R",
      description: "Czas pracy P2 — minuty",
      live: { kind: "holding", plcR: 124 },
    },
    {
      hr: 119,
      symbol: "P2_SS",
      plc: "R125",
      access: "R",
      description: "Czas pracy P2 — sekundy",
      live: { kind: "holding", plcR: 125 },
    },
    {
      hr: 120,
      symbol: "P1_STARTS",
      plc: "R126",
      access: "R",
      description: "Licznik startów pompy 1",
      live: { kind: "holding", plcR: 126 },
    },
    {
      hr: 121,
      symbol: "P2_STARTS",
      plc: "R127",
      access: "R",
      description: "Licznik startów pompy 2",
      live: { kind: "holding", plcR: 127 },
    },
  ];

  // Reserved pad to HR 150 (documentation / future SCADA tags)
  for (let hr = 122; hr <= 150; hr++) {
    rows.push({
      hr,
      symbol: `RES_${hr}`,
      plc: "—",
      access: "R",
      description: "Rezerwa (niezmapowana — identity_fallback / przyszła rozbudowa)",
      live: { kind: "reserved" },
    });
  }

  return rows;
}

/** Build Modbus Translation Matrix (only real rules — not reserved empty HR). */
export function createWaterTankModbusMap(): ModbusMapSnapshot {
  const entries: ModbusMapEntry[] = [
    bitToReg({
      id: "wt_hr100_i",
      symbol: "DI_PACK",
      plc_area: "discrete",
      plc_start: 0,
      modbus_start: 100,
      comment: "I0–I15 packed → HR100 (RO). bit0=I0 SIM_EN … bit9=I9 MAN_P2",
    }),
    bitToReg({
      id: "wt_hr101_q",
      symbol: "DO_PACK",
      plc_area: "coil",
      plc_start: 0,
      modbus_start: 101,
      comment: "Q0–Q15 packed → HR101 (RO). bit0=Q0 P1_RUN … bit4=Q4 ALM_FAIL",
    }),
    bitToReg({
      id: "wt_hr102_m0",
      symbol: "M_LO",
      plc_area: "memory_bit",
      plc_start: 0,
      modbus_start: 102,
      comment: "M0–M15 → HR102 (RO). bit2=DEMAND bit3=JOIN_P2 bit4=P1_OK bit5=P2_OK bit6=P1_CMD bit8=P2_CMD",
    }),
    bitToReg({
      id: "wt_hr103_m16",
      symbol: "M_HI",
      plc_area: "memory_bit",
      plc_start: 16,
      modbus_start: 103,
      comment: "M16–M31 → HR103 (RO). bit4=M20 ONE bit5=M21 BOTH bit6=M22 ANY bit9=M25 DRAIN",
    }),
  ];

  // Process words: HR104…121 ← R100, R101, R102, R103, R105…R107, R112…R114, R120…R127
  const process: [number, number, string, boolean, string][] = [
    [104, 100, "LEVEL_cm", false, "Level 0…1000 cm"],
    [105, 101, "K_x100", false, "Inflow K×100"],
    [106, 102, "FILL_STEP", false, "Fill cm/tick"],
    [107, 103, "PUMP_STEP", true, "Single pump capacity/tick"],
    [108, 105, "SP_STOP", false, "Stop level 200 cm"],
    [109, 106, "SP_P1_ON", false, "P1 ON 700 cm"],
    [110, 107, "SP_P2_ON", false, "P2 ON 800 cm"],
    [111, 112, "CAP", true, "Total pump capacity"],
    [112, 113, "DRAIN", true, "CAP−FILL"],
    [113, 114, "FILL_NET", true, "FILL−CAP"],
    [114, 120, "P1_HH", true, "P1 hours"],
    [115, 121, "P1_MM", true, "P1 minutes"],
    [116, 122, "P1_SS", true, "P1 seconds"],
    [117, 123, "P2_HH", true, "P2 hours"],
    [118, 124, "P2_MM", true, "P2 minutes"],
    [119, 125, "P2_SS", true, "P2 seconds"],
    [120, 126, "P1_STARTS", true, "P1 starts"],
    [121, 127, "P2_STARTS", true, "P2 starts"],
  ];

  for (const [hr, r, sym, ro, c] of process) {
    entries.push(
      direct({
        id: `wt_hr${hr}_r${r}`,
        symbol: sym,
        plc_start: r,
        modbus_start: hr,
        write_protected: ro,
        comment: `R${r} ${sym} → HR${hr}${ro ? " (RO)" : " (R/W)"} · ${c}`,
      })
    );
  }

  return {
    entries,
    // Keep identity for TV/CV banks and spare; packed I/Q live only via HR100/101 rules.
    identity_fallback: true,
    write_protect_mode: "strict",
  };
}
