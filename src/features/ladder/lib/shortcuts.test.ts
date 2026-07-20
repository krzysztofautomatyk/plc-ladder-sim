import { describe, it, expect } from "vitest";
import {
  INSTRUCTION_SHORTCUTS,
  kindForKey,
  shortcutFor,
} from "./shortcuts";

describe("instruction shortcuts", () => {
  it("maps keys to instructions (case-insensitive)", () => {
    expect(kindForKey("1")).toBe("contact_no");
    expect(kindForKey("5")).toBe("coil");
    expect(kindForKey("o")).toBe("or_branch");
    expect(kindForKey("O")).toBe("or_branch");
    expect(kindForKey("T")).toBe("ton");
  });

  it("returns undefined for unbound keys", () => {
    expect(kindForKey("z")).toBeUndefined();
    expect(kindForKey("Enter")).toBeUndefined();
  });

  it("round-trips shortcutFor / kindForKey", () => {
    for (const [key, kind] of Object.entries(INSTRUCTION_SHORTCUTS)) {
      expect(shortcutFor(kind)).toBe(key);
      expect(kindForKey(key)).toBe(kind);
    }
  });

  it("uses unique keys", () => {
    const keys = Object.keys(INSTRUCTION_SHORTCUTS);
    expect(new Set(keys).size).toBe(keys.length);
  });
});
