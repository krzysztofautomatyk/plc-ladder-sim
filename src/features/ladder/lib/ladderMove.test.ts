import { describe, it, expect } from "vitest";
import { findInRung, moveInRung, canMoveInRung } from "./ladderMove";
import type { LadderElement, RungNode } from "../../../shared/lib/types";
import { makeParallelGroup } from "./ladderEdit";

const c = (id: string, index = 0): LadderElement => ({
  type: "contact_no",
  id,
  address: { area: "discrete", index },
});
const coil = (id: string): LadderElement => ({
  type: "coil",
  id,
  address: { area: "coil", index: 0 },
});

describe("leading OR enter/exit (visual layout)", () => {
  it("‹ without branch selection → NEW bottom OR branch", () => {
    const elements: RungNode[] = [c("a"), coil("q")];
    const or = [[c("o0")], [c("o1")]];
    const out = moveInRung(elements, or, "a", "left")!;
    expect(out.or_branches).toHaveLength(3);
    expect(out.or_branches[2].map((e) => e.id)).toEqual(["a"]);
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["o0"]);
    expect(out.elements.map((n) => n.id)).toEqual(["q"]);
  });

  it("‹ with orBranchHint → AND append to that branch (right side)", () => {
    const elements: RungNode[] = [c("a"), coil("q")];
    const or = [[c("o0")], [c("o1")]];
    const out = moveInRung(elements, or, "a", "left", { orBranchHint: 0 })!;
    expect(out.or_branches).toHaveLength(2);
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["o0", "a"]);
    expect(out.or_branches[1].map((e) => e.id)).toEqual(["o1"]);
    expect(out.elements.map((n) => n.id)).toEqual(["q"]);
  });

  it("‹ with orBranchHint=1 → AND into second branch", () => {
    const elements: RungNode[] = [c("a"), coil("q")];
    const or = [[c("o0")], [c("o1")]];
    const out = moveInRung(elements, or, "a", "left", { orBranchHint: 1 })!;
    expect(out.or_branches[1].map((e) => e.id)).toEqual(["o1", "a"]);
  });

  it("› on last OR contact at max width exits to series (right of OR)", () => {
    const elements: RungNode[] = [coil("q")];
    const or = [[c("o0"), c("o1")]];
    const out = moveInRung(elements, or, "o1", "right")!;
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["o0"]);
    expect(out.elements.map((n) => n.id)).toEqual(["o1", "q"]);
  });

  it("› inside OR swaps right, does not jump out mid-branch", () => {
    const or = [[c("a"), c("b"), c("c")]];
    const out = moveInRung([], or, "a", "right")!;
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["b", "a", "c"]);
  });

  it("‹ on first OR contact does not exit (stays)", () => {
    const or = [[c("a"), c("b")]];
    const out = moveInRung([coil("q")], or, "a", "left")!;
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["a", "b"]);
  });

  it("▲ / ▼ move between OR branches keeping column", () => {
    const or = [[c("a"), c("b")], [c("c")]];
    const down = moveInRung([], or, "a", "down")!;
    expect(down.or_branches[0].map((e) => e.id)).toEqual(["b"]);
    expect(down.or_branches[1].map((e) => e.id)).toEqual(["a", "c"]);
  });

  it("› on short OR branch does NOT exit past longer sibling (no jump past OR)", () => {
    const or = [[c("short")], [c("long0"), c("long1")]];
    const out = moveInRung([coil("q")], or, "short", "right")!;
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["short"]);
    expect(out.or_branches[1].map((e) => e.id)).toEqual(["long0", "long1"]);
    expect(out.elements.map((n) => n.id)).toEqual(["q"]);
    expect(canMoveInRung([coil("q")], or, "short", "right")).toBe(false);
  });

  it("› on long OR branch last cell DOES exit at block edge", () => {
    const or = [[c("short")], [c("long0"), c("long1")]];
    const out = moveInRung([coil("q")], or, "long1", "right")!;
    expect(out.or_branches.flat().map((e) => e.id)).not.toContain("long1");
    expect(out.elements.map((n) => n.id)).toEqual(["long1", "q"]);
  });

  it("‹ prefers inline parallel neighbor over leading OR", () => {
    const g = makeParallelGroup("g", [[c("p0")], [c("p1")]]);
    const elements: RungNode[] = [g, c("a"), coil("q")];
    const or = [[c("o")]];
    const out = moveInRung(elements, or, "a", "left")!;
    const group = out.elements.find((n) => n.type === "parallel");
    expect(group && group.type === "parallel").toBe(true);
    if (group && group.type === "parallel") {
      expect(group.branches).toHaveLength(3);
      expect(group.branches[2].map((e) => e.id)).toEqual(["a"]);
    }
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["o"]);
  });
});

describe("inline parallel — no jump over", () => {
  it("› without selection → NEW bottom ∥ branch", () => {
    const g = makeParallelGroup("g", [[c("p0")], [c("p1")]]);
    const elements: RungNode[] = [c("a"), g, c("b"), coil("q")];
    const out = moveInRung(elements, [], "a", "right")!;
    const group = out.elements.find((n) => n.type === "parallel");
    expect(group && group.type === "parallel").toBe(true);
    if (group && group.type === "parallel") {
      expect(group.branches).toHaveLength(3);
      expect(group.branches[2].map((e) => e.id)).toEqual(["a"]);
    }
  });

  it("› with parBranchHint → AND into that ∥ branch", () => {
    const g = makeParallelGroup("g", [[c("p0")], [c("p1")]]);
    const elements: RungNode[] = [c("a"), g, c("b"), coil("q")];
    const out = moveInRung(elements, [], "a", "right", {
      parBranchHint: 0,
      parGroupHint: "g",
    })!;
    const group = out.elements.find((n) => n.type === "parallel");
    expect(group && group.type === "parallel").toBe(true);
    if (group && group.type === "parallel") {
      expect(group.branches).toHaveLength(2);
      expect(group.branches[0].map((e) => e.id)).toEqual(["p0", "a"]);
      expect(group.branches[1].map((e) => e.id)).toEqual(["p1"]);
    }
  });

  it("‹ from first element in parallel exits before group", () => {
    const g = makeParallelGroup("g", [[c("p0"), c("p1")]]);
    const elements: RungNode[] = [c("x"), g, coil("q")];
    const out = moveInRung(elements, [], "p0", "left")!;
    expect(out.elements.map((n) => n.id)).toContain("p0");
    const ids = out.elements.map((n) => n.id);
    expect(ids.indexOf("p0")).toBeLessThan(ids.indexOf("g"));
  });

  it("› from last element in parallel exits after group when at max width", () => {
    const g = makeParallelGroup("g", [[c("p0"), c("p1")]]);
    const elements: RungNode[] = [g, c("y"), coil("q")];
    const out = moveInRung(elements, [], "p1", "right")!;
    const ids = out.elements.map((n) => n.id);
    expect(ids.indexOf("g")).toBeLessThan(ids.indexOf("p1"));
  });

  it("› on short parallel branch does not jump past longer sibling", () => {
    const g = makeParallelGroup("g", [[c("s")], [c("l0"), c("l1")]]);
    const elements: RungNode[] = [c("x"), g, coil("q")];
    const out = moveInRung(elements, [], "s", "right")!;
    const group = out.elements.find((n) => n.type === "parallel");
    expect(group && group.type === "parallel").toBe(true);
    if (group && group.type === "parallel") {
      expect(group.branches[0].map((e) => e.id)).toEqual(["s"]);
      expect(group.branches[1].map((e) => e.id)).toEqual(["l0", "l1"]);
    }
  });

  it("water-tank P1: M4 › AND into branch 0 when selected", () => {
    const g = makeParallelGroup("g", [[c("m2")], [c("i8")]]);
    const elements: RungNode[] = [c("m4"), g, coil("m6")];
    const out = moveInRung(elements, [], "m4", "right", {
      parBranchHint: 0,
      parGroupHint: "g",
    })!;
    const group = out.elements.find((n) => n.type === "parallel") as {
      type: "parallel";
      branches: LadderElement[][];
    };
    expect(group.branches[0].map((e) => e.id)).toEqual(["m2", "m4"]);
  });
});

describe("coils vertical stack", () => {
  it("▲ / ▼ reorder coils", () => {
    const elements: RungNode[] = [c("a"), coil("q0"), coil("q1")];
    const up = moveInRung(elements, [], "q1", "up")!;
    expect(up.elements.map((n) => n.id)).toEqual(["a", "q1", "q0"]);
    const down = moveInRung(up.elements, [], "q1", "down")!;
    expect(down.elements.map((n) => n.id)).toEqual(["a", "q0", "q1"]);
  });
});

describe("canMoveInRung", () => {
  it("disables left on first OR contact", () => {
    const or = [[c("a")]];
    expect(canMoveInRung([coil("q")], or, "a", "left")).toBe(false);
    expect(canMoveInRung([coil("q")], or, "a", "right")).toBe(true);
  });

  it("disables › on short OR branch (uneven)", () => {
    const or = [[c("s")], [c("a"), c("b")]];
    expect(canMoveInRung([coil("q")], or, "s", "right")).toBe(false);
    expect(canMoveInRung([coil("q")], or, "b", "right")).toBe(true);
  });
});

describe("findInRung", () => {
  it("finds series, or, and parallel locations", () => {
    const g = makeParallelGroup("g", [[c("p")]]);
    const elements: RungNode[] = [c("s"), g, coil("q")];
    const or = [[c("o")]];
    expect(findInRung(elements, or, "s")?.kind).toBe("series");
    expect(findInRung(elements, or, "o")?.kind).toBe("or");
    expect(findInRung(elements, or, "p")?.kind).toBe("par");
  });
});

describe("demo network 0 style", () => {
  it("‹ on series without selection → new bottom OR branch", () => {
    const elements: RungNode[] = [c("i1"), coil("q0")];
    const or = [[c("i0")], [c("q0seal")]];
    const out = moveInRung(elements, or, "i1", "left")!;
    expect(out.or_branches).toHaveLength(3);
    expect(out.or_branches[2].map((e) => e.id)).toEqual(["i1"]);
  });

  it("‹ on series with OR0 selected → AND into OR0", () => {
    const elements: RungNode[] = [c("i1"), coil("q0")];
    const or = [[c("i0")], [c("q0seal")]];
    const out = moveInRung(elements, or, "i1", "left", { orBranchHint: 0 })!;
    expect(out.or_branches[0].map((e) => e.id)).toEqual(["i0", "i1"]);
    expect(out.elements.map((n) => n.id)).toEqual(["q0"]);
  });

  it("› on OR i0 (equal width 1) exits to series", () => {
    const elements: RungNode[] = [c("i1"), coil("q0")];
    const or = [[c("i0")], [c("q0seal")]];
    const out = moveInRung(elements, or, "i0", "right")!;
    expect(out.or_branches.map((b) => b.map((e) => e.id))).toEqual([["q0seal"]]);
    expect(out.elements.map((n) => n.id)).toEqual(["i0", "i1", "q0"]);
  });
});
