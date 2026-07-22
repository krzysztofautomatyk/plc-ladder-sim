import { describe, it, expect } from "vitest";
import {
  createWaterTankModbusMap,
  waterTankRegisterTable,
} from "./waterTankModbusMap";

describe("createWaterTankModbusMap (HR100–150)", () => {
  it("packs I/Q/M into HR100–103 and process from HR104", () => {
    const m = createWaterTankModbusMap();
    const byHr = (hr: number) =>
      m.entries.find((e) => e.modbus_table === "holding" && e.modbus_start === hr);

    expect(byHr(100)?.mapping_type).toBe("bit_to_register");
    expect(byHr(100)?.plc_area).toBe("discrete");
    expect(byHr(101)?.plc_area).toBe("coil");
    expect(byHr(102)?.plc_area).toBe("memory_bit");
    expect(byHr(102)?.plc_start).toBe(0);
    expect(byHr(103)?.plc_start).toBe(16);

    const level = byHr(104);
    expect(level?.mapping_type).toBe("direct");
    expect(level?.plc_start).toBe(100);
    expect(level?.symbol_name).toBe("LEVEL_cm");

    const sp = byHr(108);
    expect(sp?.plc_start).toBe(105); // SP_STOP
  });

  it("keeps all mapped HR addresses in 100…121", () => {
    const m = createWaterTankModbusMap();
    for (const e of m.entries) {
      if (e.modbus_table !== "holding") continue;
      expect(e.modbus_start).toBeGreaterThanOrEqual(100);
      expect(e.modbus_start).toBeLessThanOrEqual(121);
    }
  });

  it("has no overlapping holding addresses", () => {
    const m = createWaterTankModbusMap();
    const seen = new Set<number>();
    for (const e of m.entries) {
      if (e.modbus_table !== "holding") continue;
      expect(seen.has(e.modbus_start), `dup HR${e.modbus_start}`).toBe(false);
      seen.add(e.modbus_start);
    }
  });
});

describe("waterTankRegisterTable", () => {
  it("covers HR 100…150 inclusive", () => {
    const t = waterTankRegisterTable();
    expect(t[0].hr).toBe(100);
    expect(t[t.length - 1].hr).toBe(150);
    expect(t).toHaveLength(51);
  });

  it("documents bit packs on HR100–103", () => {
    const t = waterTankRegisterTable();
    expect(t.find((r) => r.hr === 100)?.bits?.length).toBe(16);
    expect(t.find((r) => r.hr === 101)?.bits?.some((b) => b.tag === "Q0")).toBe(true);
    expect(t.find((r) => r.hr === 102)?.bits?.some((b) => b.tag === "M2")).toBe(true);
  });
});
