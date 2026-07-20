import { describe, it, expect } from "vitest";
import {
  isRightRailElement,
  splitRungElements,
  seriesPowerIn,
  seriesPowerOut,
  rungColumns,
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
  const conducts = (n: { id: string }) => activeIds.has(n.id);

  it("feeds the left rail into index 0", () => {
    expect(seriesPowerIn(chain, 0, conducts)).toBe(true);
  });

  it("propagates power from the previous conducting element", () => {
    expect(seriesPowerIn(chain, 1, conducts)).toBe(true); // a active
    expect(seriesPowerIn(chain, 2, conducts)).toBe(true); // b active
  });

  it("stops power after a non-conducting element", () => {
    const onlyA = (n: { id: string }) => n.id === "a";
    expect(seriesPowerIn(chain, 2, onlyA)).toBe(false); // b inactive
  });

  it("reports the outgoing wire state per element", () => {
    expect(seriesPowerOut(chain, 0, conducts)).toBe(true); // a active
    expect(seriesPowerOut(chain, 2, conducts)).toBe(false); // c inactive
  });
});

describe("rungColumns", () => {
  it("splits a plain series rung into series + coils with no parallel block", () => {
    const cols = rungColumns({
      elements: [contact("x4"), coil("m1")],
      or_branches: [],
    });
    expect(cols.hasParallel).toBe(false);
    expect(cols.branches).toEqual([]);
    expect(cols.series.map((e) => e.id)).toEqual(["x4"]);
    expect(cols.coils.map((e) => e.id)).toEqual(["m1"]);
  });

  it("models (X1 OR X2 OR X3) AND X4 -> M1 like the reference ladder", () => {
    const cols = rungColumns({
      elements: [contact("x4"), coil("m1")],
      or_branches: [[contact("x1")], [contact("x2")], [contact("x3")]],
    });
    expect(cols.hasParallel).toBe(true);
    expect(cols.branches).toHaveLength(3);
    expect(cols.branches.map((b) => b[0].id)).toEqual(["x1", "x2", "x3"]);
    expect(cols.series.map((e) => e.id)).toEqual(["x4"]);
    expect(cols.coils.map((e) => e.id)).toEqual(["m1"]);
  });

  it("tolerates a missing or_branches field", () => {
    const cols = rungColumns({ elements: [coil("q")] });
    expect(cols.hasParallel).toBe(false);
    expect(cols.coils.map((e) => e.id)).toEqual(["q"]);
  });
});
