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
    expect(prefixToArea("M")).toBe("holding");
    expect(prefixToArea("R")).toBe("holding");
  });

  it("maps areas back to prefixes", () => {
    expect(areaToPrefix("discrete", false)).toBe("I");
    expect(areaToPrefix("coil", false)).toBe("Q");
    expect(areaToPrefix("holding", true)).toBe("M");
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

  it("treats a bare M marker as bit 0 of a holding word", () => {
    const m = parseVarString("M5");
    expect(m?.address.area).toBe("holding");
    expect(m?.address.bit).toBe(0);
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

  it("forces a bit for M markers", () => {
    expect(formToAddress("M", 5, 2, false)).toEqual({
      area: "holding",
      index: 5,
      bit: 2,
    });
  });
});
