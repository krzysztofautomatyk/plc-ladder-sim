import { describe, it, expect } from "vitest";
import { createWaterTankProgram, WATER_TANK_MEMORY_SEED } from "./waterTankProgram";

describe("createWaterTankProgram", () => {
  it("builds a named dual-pump station with many networks", () => {
    const p = createWaterTankProgram();
    expect(p.name).toBe("Water_Tank_Dual_Pump");
    expect(p.version).toBe("1.0.0");
    expect(p.rungs.length).toBeGreaterThan(25);
    expect(p.rungs.every((r) => r.enabled && r.comment.length > 0)).toBe(true);
  });

  it("covers sim, control, failover and statistics sections", () => {
    const comments = createWaterTankProgram().rungs.map((r) => r.comment);
    expect(comments.some((c) => c.includes("[SIM]"))).toBe(true);
    expect(comments.some((c) => c.includes("[CTRL]"))).toBe(true);
    expect(comments.some((c) => c.includes("[P1]"))).toBe(true);
    expect(comments.some((c) => c.includes("[P2]"))).toBe(true);
    expect(comments.some((c) => c.includes("[STAT]"))).toBe(true);
    expect(comments.some((c) => c.includes("FAULT"))).toBe(true);
  });

  it("seeds K=1.5 stress case and sim enable", () => {
    expect(WATER_TANK_MEMORY_SEED.holdings[101]).toBe(150);
    expect(WATER_TANK_MEMORY_SEED.holdings[100]).toBe(500);
    expect(WATER_TANK_MEMORY_SEED.holdings[108]).toBe(100);
    expect(WATER_TANK_MEMORY_SEED.discretes[0]).toBe(true);
  });

  it("is immutable across calls", () => {
    const a = createWaterTankProgram();
    const b = createWaterTankProgram();
    a.rungs[0].comment = "mutated";
    expect(b.rungs[0].comment).not.toBe("mutated");
  });
});
