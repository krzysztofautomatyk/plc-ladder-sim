import { describe, it, expect } from "vitest";
import { createDemoProgram, uid } from "./demoProgram";

describe("createDemoProgram", () => {
  it("mirrors the Rust demo (name, version, four rungs)", () => {
    const p = createDemoProgram();
    expect(p.name).toBe("Demo_Start_Stop");
    expect(p.version).toBe("2.0.0");
    expect(p.rungs).toHaveLength(4);
  });

  it("keeps the OR seal-in branches on the first rung", () => {
    const p = createDemoProgram();
    expect(p.rungs[0].or_branches).toHaveLength(2);
    expect(p.rungs[0].elements.at(-1)).toMatchObject({
      type: "coil",
      address: { area: "coil", index: 0 },
    });
  });

  it("produces a fresh object each call (no shared mutation)", () => {
    const a = createDemoProgram();
    const b = createDemoProgram();
    a.rungs[0].comment = "changed";
    expect(b.rungs[0].comment).not.toBe("changed");
  });
});

describe("uid", () => {
  it("applies the requested prefix", () => {
    expect(uid("rung")).toMatch(/^rung_[a-z0-9]+$/);
  });

  it("is effectively unique across calls", () => {
    const ids = new Set(Array.from({ length: 500 }, () => uid()));
    expect(ids.size).toBe(500);
  });
});
