import { describe, it, expect } from "vitest";
import {
  isRightRailElement,
  splitRungElements,
  seriesPowerIn,
  seriesPowerOut,
} from "./ladderLayout";
import type { LadderElement } from "../../../shared/lib/types";

const contact = (id: string): LadderElement => ({
  type: "contact_no",
  id,
  address: { area: "discrete", index: 0 },
});
const coil = (id: string): LadderElement => ({
  type: "coil",
  id,
  address: { area: "coil", index: 0 },
});

describe("isRightRailElement", () => {
  it("classifies coils as right-rail outputs", () => {
    expect(isRightRailElement(coil("q"))).toBe(true);
    expect(
      isRightRailElement({
        type: "coil_set",
        id: "s",
        address: { area: "coil", index: 0 },
      })
    ).toBe(true);
  });

  it("classifies contacts as left-rail logic", () => {
    expect(isRightRailElement(contact("c"))).toBe(false);
  });
});

describe("splitRungElements", () => {
  it("separates series logic from output coils", () => {
    const { left, right } = splitRungElements([
      contact("c1"),
      contact("c2"),
      coil("q1"),
    ]);
    expect(left.map((e) => e.id)).toEqual(["c1", "c2"]);
    expect(right.map((e) => e.id)).toEqual(["q1"]);
  });
});

describe("series power flow", () => {
  const chain = [contact("a"), contact("b"), contact("c")];
  const activeIds = new Set(["a", "b"]);
  const isActive = (id: string) => activeIds.has(id);

  it("feeds the left rail into index 0", () => {
    expect(seriesPowerIn(chain, 0, isActive)).toBe(true);
  });

  it("propagates power from the previous conducting element", () => {
    expect(seriesPowerIn(chain, 1, isActive)).toBe(true); // a active
    expect(seriesPowerIn(chain, 2, isActive)).toBe(true); // b active
  });

  it("stops power after a non-conducting element", () => {
    const isActiveOnlyA = (id: string) => id === "a";
    expect(seriesPowerIn(chain, 2, isActiveOnlyA)).toBe(false); // b inactive
  });

  it("reports the outgoing wire state per element", () => {
    expect(seriesPowerOut(chain, 0, isActive)).toBe(true); // a active
    expect(seriesPowerOut(chain, 2, isActive)).toBe(false); // c inactive
  });
});
