import { describe, it, expect } from "vitest";
import { resolveStrokes, COL_GREEN, COL_BLUE, COL_OFF } from "./strokeColors";

describe("resolveStrokes", () => {
  it("shows everything de-energised when off", () => {
    const s = resolveStrokes({
      active: false,
      powerIn: false,
      energized: false,
      isCoil: false,
    });
    expect(s.strokeIn).toBe(COL_OFF);
    expect(s.strokeOut).toBe(COL_OFF);
    expect(s.strokeBody).toBe(COL_OFF);
    expect(s.fillCoil).toBe("none");
  });

  it("lights the input wire green when power flows in", () => {
    const s = resolveStrokes({
      active: false,
      powerIn: true,
      energized: false,
      isCoil: false,
    });
    expect(s.strokeIn).toBe(COL_GREEN);
  });

  it("draws a conducting contact fully green", () => {
    const s = resolveStrokes({
      active: true,
      powerIn: true,
      energized: false,
      isCoil: false,
    });
    expect(s.strokeIn).toBe(COL_GREEN);
    expect(s.strokeOut).toBe(COL_GREEN);
    expect(s.strokeBody).toBe(COL_GREEN);
  });

  it("draws an energised coil blue with a translucent fill", () => {
    const s = resolveStrokes({
      active: false,
      powerIn: true,
      energized: true,
      isCoil: true,
    });
    expect(s.strokeOut).toBe(COL_BLUE);
    expect(s.strokeBody).toBe(COL_BLUE);
    expect(s.fillCoil).toContain("rgba(21, 101, 192");
  });

  it("never paints a contact blue just because its bit is ON", () => {
    // Bug fix: bit true must not turn contact poles / stubs blue
    const s = resolveStrokes({
      active: true,
      powerIn: true,
      energized: true,
      isCoil: false,
    });
    expect(s.strokeOut).toBe(COL_GREEN);
    expect(s.strokeBody).toBe(COL_GREEN);
  });

  it("contact with bit ON but no power stays off-colored (not blue)", () => {
    const s = resolveStrokes({
      active: false,
      powerIn: false,
      energized: true,
      isCoil: false,
    });
    expect(s.strokeOut).toBe(COL_OFF);
    expect(s.strokeBody).toBe(COL_OFF);
  });
});

