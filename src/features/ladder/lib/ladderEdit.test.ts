import { describe, it, expect } from "vitest";
import {
  addParallelBranch,
  addToParallelBranch,
  insertBeforeCoils,
  isCoilNode,
  makeParallelGroup,
  removeNodeById,
  removeParallelBranch,
  updateNodeById,
} from "./ladderEdit";
import type { LadderElement, RungNode } from "../../../shared/lib/types";

const contact = (id: string, index = 0): LadderElement => ({
  type: "contact_no",
  id,
  address: { area: "discrete", index },
});
const coil = (id: string): LadderElement => ({
  type: "coil",
  id,
  address: { area: "coil", index: 0 },
});

describe("insertBeforeCoils", () => {
  it("inserts contacts before the output coil", () => {
    const nodes: RungNode[] = [contact("a"), coil("q")];
    const out = insertBeforeCoils(nodes, contact("b"));
    expect(out.map((n) => n.id)).toEqual(["a", "b", "q"]);
  });

  it("appends coils at the end", () => {
    const nodes: RungNode[] = [contact("a")];
    const out = insertBeforeCoils(nodes, coil("q"));
    expect(out.map((n) => n.id)).toEqual(["a", "q"]);
  });

  it("treats a parallel group as a series node (not a coil)", () => {
    const group = makeParallelGroup("g", [[contact("b")]]);
    expect(isCoilNode(group)).toBe(false);
    const out = insertBeforeCoils([coil("q")], group);
    expect(out.map((n) => n.id)).toEqual(["g", "q"]);
  });
});

describe("updateNodeById", () => {
  it("updates a top-level element", () => {
    const nodes: RungNode[] = [contact("a"), coil("q")];
    const out = updateNodeById(nodes, { ...contact("a", 5) });
    const a = out[0] as LadderElement;
    expect(a.type === "contact_no" && a.address.index).toBe(5);
  });

  it("updates an element inside a parallel branch", () => {
    const nodes: RungNode[] = [makeParallelGroup("g", [[contact("b")], [contact("c")]]), coil("q")];
    const out = updateNodeById(nodes, { ...contact("c", 9) });
    const group = out[0];
    if (group.type !== "parallel") throw new Error("expected group");
    expect(group.branches[1][0]).toMatchObject({ id: "c", address: { index: 9 } });
  });
});

describe("removeNodeById", () => {
  it("removes a top-level element", () => {
    const out = removeNodeById([contact("a"), coil("q")], "a");
    expect(out.map((n) => n.id)).toEqual(["q"]);
  });

  it("removes an element from a parallel branch and prunes the empty branch", () => {
    const nodes: RungNode[] = [makeParallelGroup("g", [[contact("b")], [contact("c")]]), coil("q")];
    const out = removeNodeById(nodes, "c");
    const group = out[0];
    if (group.type !== "parallel") throw new Error("expected group");
    expect(group.branches).toHaveLength(1);
  });

  it("drops the whole group when its last branch empties", () => {
    const nodes: RungNode[] = [makeParallelGroup("g", [[contact("b")]]), coil("q")];
    const out = removeNodeById(nodes, "b");
    expect(out.map((n) => n.id)).toEqual(["q"]);
  });

  it("removes a whole group by its id", () => {
    const nodes: RungNode[] = [makeParallelGroup("g", [[contact("b")]]), coil("q")];
    const out = removeNodeById(nodes, "g");
    expect(out.map((n) => n.id)).toEqual(["q"]);
  });
});

describe("parallel branch ops", () => {
  it("adds a branch and appends to a branch", () => {
    let nodes: RungNode[] = [makeParallelGroup("g", [[contact("b")]])];
    nodes = addParallelBranch(nodes, "g", contact("c"));
    let group = nodes[0];
    if (group.type !== "parallel") throw new Error("expected group");
    expect(group.branches).toHaveLength(2);

    nodes = addToParallelBranch(nodes, "g", 0, contact("d"));
    group = nodes[0];
    if (group.type !== "parallel") throw new Error("expected group");
    expect(group.branches[0].map((e) => e.id)).toEqual(["b", "d"]);
  });

  it("removes a branch and prunes an emptied group", () => {
    const nodes: RungNode[] = [makeParallelGroup("g", [[contact("b")], [contact("c")]])];
    const one = removeParallelBranch(nodes, "g", 0);
    const group = one[0];
    if (group.type !== "parallel") throw new Error("expected group");
    expect(group.branches).toHaveLength(1);

    const gone = removeParallelBranch(one, "g", 0);
    expect(gone).toHaveLength(0);
  });
});
