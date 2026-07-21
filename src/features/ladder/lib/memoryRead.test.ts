import { describe, it, expect } from "vitest";
import {
  readMemoryBit,
  readMemoryValue,
  formatLiveOperand,
  elementAddress,
} from "./memoryRead";
import type { MemorySnapshot } from "../../../shared/lib/types";

function mem(partial: Partial<MemorySnapshot>): MemorySnapshot {
  return {
    coils: [],
    discrete_inputs: [],
    holding_registers: [],
    input_registers: [],
    memory_bits: [],
    memory_words: [],
    timer_et: [],
    timer_q: [],
    counter_cv: [],
    counter_q: [],
    run_state: "stop",
    scan_count: 0,
    last_scan_us: 0,
    cycle_ms: 20,
    program_hash: "",
    program_version: "",
    fault_code: 0,
    fault_message: "",
    ...partial,
  };
}

describe("readMemoryBit", () => {
  it("returns false for a missing snapshot", () => {
    expect(readMemoryBit(null, { area: "coil", index: 0 })).toBe(false);
  });

  it("reads coils and discrete inputs", () => {
    const m = mem({ coils: [false, true], discrete_inputs: [true] });
    expect(readMemoryBit(m, { area: "coil", index: 1 })).toBe(true);
    expect(readMemoryBit(m, { area: "coil", index: 0 })).toBe(false);
    expect(readMemoryBit(m, { area: "discrete", index: 0 })).toBe(true);
  });

  it("reads whole-word truthiness for holding registers", () => {
    const m = mem({ holding_registers: [0, 5] });
    expect(readMemoryBit(m, { area: "holding", index: 1 })).toBe(true);
    expect(readMemoryBit(m, { area: "holding", index: 0 })).toBe(false);
  });

  it("reads a specific bit inside a holding word", () => {
    const m = mem({ holding_registers: [0b0000_1000] });
    expect(readMemoryBit(m, { area: "holding", index: 0, bit: 3 })).toBe(true);
    expect(readMemoryBit(m, { area: "holding", index: 0, bit: 2 })).toBe(false);
  });

  it("reads input register bits", () => {
    const m = mem({ input_registers: [0b0100] });
    expect(readMemoryBit(m, { area: "input_reg", index: 0, bit: 2 })).toBe(true);
    expect(readMemoryBit(m, { area: "input_reg", index: 0, bit: 0 })).toBe(false);
  });
});

describe("readMemoryValue / formatLiveOperand", () => {
  it("reads holding words for MOVE-style display", () => {
    const m = mem({ holding_registers: Array(50).fill(0).map((_, i) => (i === 40 ? 1234 : 0)) });
    expect(readMemoryValue(m, { area: "holding", index: 40 })).toBe(1234);
    expect(formatLiveOperand(m, { area: "holding", index: 40 }, (a) => `R${a!.index}`)).toBe(
      "R40=1234"
    );
  });

  it("reads timer ET from dedicated image", () => {
    const et = Array(16).fill(0);
    et[0] = 850;
    const m = mem({ timer_et: et });
    expect(readMemoryValue(m, { area: "holding", index: 2048 })).toBe(850);
  });
});

describe("elementAddress", () => {
  it("returns the address when present", () => {
    expect(elementAddress({ address: { area: "coil", index: 1 } })).toEqual({
      area: "coil",
      index: 1,
    });
  });

  it("returns null for elements without an address", () => {
    expect(elementAddress({ type: "wire" })).toBeNull();
  });
});

