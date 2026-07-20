import { describe, it, expect } from "vitest";
import { newId } from "./id";

describe("newId", () => {
  it("uses the given prefix", () => {
    expect(newId("c")).toMatch(/^c_[a-z0-9]+$/);
    expect(newId("ton")).toMatch(/^ton_[a-z0-9]+$/);
  });

  it("generates unique ids", () => {
    const ids = new Set(Array.from({ length: 1000 }, () => newId("e")));
    expect(ids.size).toBe(1000);
  });
});
