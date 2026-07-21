import { describe, it, expect } from "vitest";
import {
  areaToPrefix,
  formatAddress,
  formToAddress,
  parseVarString,
  prefixToArea,
} from "./addressFormat";

describe("formatAddress", () => {
  it("formats each memory area", () => {
    expect(formatAddress({ area: "discrete", index: 0 })).toBe("I0");
    expect(formatAddress({ area: "coil", index: 3 })).toBe("Q3");
    expect(formatAddress({ area: "holding", index: 10 })).toBe("R10");
    expect(formatAddress({ area: "input_reg", index: 4 })).toBe("IW4");
    expect(formatAddress({ area: "memory_bit", index: 5 })).toBe("M5");
    expect(formatAddress({ area: "memory_word", index: 2 })).toBe("MR2");
    expect(formatAddress({ area: "memory_word", index: 2, bit: 3 })).toBe("MR2.3");
  });

  it("includes the bit when present", () => {
    expect(formatAddress({ area: "discrete", index: 5, bit: 2 })).toBe("I5.2");
    expect(formatAddress({ area: "holding", index: 1, bit: 3 })).toBe("R1.3");
  });

  it("renders a placeholder for missing addresses", () => {
    expect(formatAddress(null)).toBe("—");
    expect(formatAddress(undefined)).toBe("—");
  });
});

describe("prefix / area mapping", () => {
  it("maps prefixes to areas", () => {
    expect(prefixToArea("I")).toBe("discrete");
    expect(prefixToArea("Q")).toBe("coil");
    expect(prefixToArea("M")).toBe("memory_bit");
    expect(prefixToArea("MR")).toBe("memory_word");
    expect(prefixToArea("R")).toBe("holding");
  });

  it("maps areas back to prefixes", () => {
    expect(areaToPrefix("discrete", false)).toBe("I");
    expect(areaToPrefix("coil", false)).toBe("Q");
    expect(areaToPrefix("memory_bit", false)).toBe("M");
    expect(areaToPrefix("memory_word", true)).toBe("MR");
    expect(areaToPrefix("holding", false)).toBe("R");
  });
});

describe("parseVarString", () => {
  it("parses simple bit addresses", () => {
    const i0 = parseVarString("I0");
    expect(i0?.address).toEqual({ area: "discrete", index: 0 });
    expect(i0?.display).toBe("I0");

    const q3 = parseVarString("Q3");
    expect(q3?.address).toEqual({ area: "coil", index: 3 });
  });

  it("accepts the %-prefixed IEC form and explicit bit", () => {
    const parsed = parseVarString("%I0.0");
    expect(parsed?.address.area).toBe("discrete");
    expect(parsed?.address.bit).toBe(0);
  });

  it("parses register bit syntax R1.5", () => {
    const r = parseVarString("R1.5");
    expect(r?.address).toEqual({ area: "holding", index: 1, bit: 5 });
    expect(r?.display).toBe("R1.5");
  });

  it("normalises MW/IW aliases to R words", () => {
    const mw = parseVarString("MW20");
    expect(mw?.address).toEqual({ area: "holding", index: 20 });
    expect(mw?.display).toBe("R20");
  });

  it("treats a bare M marker as an internal marker bit (never Modbus)", () => {
    const m = parseVarString("M5");
    expect(m?.address).toEqual({ area: "memory_bit", index: 5 });
    expect(m?.display).toBe("M5");
  });

  it("parses internal memory registers MR and MR.bit", () => {
    expect(parseVarString("MR20")?.address).toEqual({ area: "memory_word", index: 20 });
    expect(parseVarString("MR1.5")?.address).toEqual({
      area: "memory_word",
      index: 1,
      bit: 5,
    });
  });

  it("rejects invalid input", () => {
    expect(parseVarString("garbage")).toBeNull();
    expect(parseVarString("")).toBeNull();
    expect(parseVarString("R1.16")).toBeNull(); // bit out of 0..15
    expect(parseVarString("Z5")).toBeNull();
  });
});

describe("formToAddress", () => {
  it("builds bitless addresses for I/Q", () => {
    expect(formToAddress("I", 0, null, false)).toEqual({
      area: "discrete",
      index: 0,
    });
    expect(formToAddress("Q", 2, null, false)).toEqual({
      area: "coil",
      index: 2,
    });
  });

  it("keeps register bits within 0..15", () => {
    expect(formToAddress("R", 1, 3, false)).toEqual({
      area: "holding",
      index: 1,
      bit: 3,
    });
    expect(formToAddress("R", 1, 99, true)).toEqual({
      area: "holding",
      index: 1,
      bit: 15,
    });
  });

  it("builds a bitless internal marker for M", () => {
    expect(formToAddress("M", 5, 2, false)).toEqual({
      area: "memory_bit",
      index: 5,
    });
  });

  it("supports bit access on internal registers MR", () => {
    expect(formToAddress("MR", 3, 7, false)).toEqual({
      area: "memory_word",
      index: 3,
      bit: 7,
    });
    expect(formToAddress("MR", 3, null, false)).toEqual({
      area: "memory_word",
      index: 3,
    });
  });
});
