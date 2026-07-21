import type { LadderProgram } from "./types";

/** Client-side copy of the Rust demo program (v2: OR seal-in + compare/move). */
export function createDemoProgram(): LadderProgram {
  return {
    name: "Demo_Start_Stop",
    version: "2.0.0",
    description: "OR seal-in, TON, CTU, compare, MOVE demo",
    rungs: [
      {
        id: "rung_0",
        comment: "(I0 OR Q0) AND NOT I1 → Q0  [OR branches]",
        enabled: true,
        or_branches: [
          [{ type: "contact_no", id: "c_i0", address: { area: "discrete", index: 0 } }],
          [{ type: "contact_no", id: "c_q0_seal", address: { area: "coil", index: 0 } }],
        ],
        elements: [
          { type: "contact_nc", id: "c_i1_stop", address: { area: "discrete", index: 1 } },
          { type: "coil", id: "coil_q0", address: { area: "coil", index: 0 } },
        ],
      },
      {
        id: "rung_1",
        comment: "TON 2000 ms when Q0 → Q1",
        enabled: true,
        or_branches: [],
        elements: [
          { type: "contact_no", id: "c_q0_ton", address: { area: "coil", index: 0 } },
          {
            type: "ton",
            id: "ton_0",
            preset_ms: 2000,
            timer_index: 0,
            done_address: { area: "coil", index: 1 },
          },
        ],
      },
      {
        id: "rung_2",
        comment: "CTU on I2, preset 5 → Q2; reset I3",
        enabled: true,
        or_branches: [],
        elements: [
          { type: "contact_no", id: "c_i2", address: { area: "discrete", index: 2 } },
          {
            type: "ctu",
            id: "ctu_0",
            preset: 5,
            counter_index: 10,
            done_address: { area: "coil", index: 2 },
            reset_address: { area: "discrete", index: 3 },
          },
        ],
      },
      {
        id: "rung_3",
        comment: "If R40 >= R41 then MOVE R40→R42 and Q3",
        enabled: true,
        or_branches: [],
        elements: [
          {
            type: "compare",
            id: "cmp_ge",
            op: "ge",
            a: { area: "holding", index: 40 },
            b: { area: "holding", index: 41 },
          },
          {
            type: "move",
            id: "mov_0",
            source: { area: "holding", index: 40 },
            dest: { area: "holding", index: 42 },
          },
          { type: "coil", id: "coil_q3", address: { area: "coil", index: 3 } },
        ],
      },
    ],
    metadata: {
      author: "system",
      standard: "IEC 61131-3 LAD subset",
      features: "or,rto,ctd,move,compare",
    },
  };
}

export function uid(prefix = "el"): string {
  return `${prefix}_${Math.random().toString(36).slice(2, 9)}`;
}
