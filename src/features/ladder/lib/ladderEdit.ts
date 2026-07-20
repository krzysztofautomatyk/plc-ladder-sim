/**
 * Pure, immutable editing operations on a rung's series (RungNode[]).
 * Handles top-level elements and inline parallel groups uniformly so the
 * store stays thin and the logic is unit-testable.
 */
import type { LadderElement, ParallelNode, RungNode } from "../../../shared/lib/types";

const COIL_TYPES = new Set(["coil", "coil_negated", "coil_set", "coil_reset"]);

export function isCoilNode(n: RungNode): boolean {
  return n.type !== "parallel" && COIL_TYPES.has(n.type);
}

/** Insert a node into the series before the first coil (coils stay right-aligned). */
export function insertBeforeCoils(nodes: RungNode[], node: RungNode): RungNode[] {
  const coilIdx = nodes.findIndex(isCoilNode);
  if (coilIdx === -1 || isCoilNode(node)) return [...nodes, node];
  const next = [...nodes];
  next.splice(coilIdx, 0, node);
  return next;
}

/** Replace an element by id anywhere (top level or inside a parallel branch). */
export function updateNodeById(nodes: RungNode[], element: LadderElement): RungNode[] {
  return nodes.map((n) => {
    if (n.type === "parallel") {
      return {
        ...n,
        branches: n.branches.map((b) =>
          b.map((e) => (e.id === element.id ? element : e))
        ),
      };
    }
    return n.id === element.id ? element : n;
  });
}

/** Remove an element (or whole parallel group) by id; prune empty branches/groups. */
export function removeNodeById(nodes: RungNode[], id: string): RungNode[] {
  const out: RungNode[] = [];
  for (const n of nodes) {
    if (n.type === "parallel") {
      if (n.id === id) continue; // remove the whole group
      const branches = n.branches
        .map((b) => b.filter((e) => e.id !== id))
        .filter((b) => b.length > 0);
      if (branches.length > 0) out.push({ ...n, branches });
    } else if (n.id !== id) {
      out.push(n);
    }
  }
  return out;
}

/** Append a new branch (seeded with one contact) to a parallel group. */
export function addParallelBranch(
  nodes: RungNode[],
  groupId: string,
  seed: LadderElement
): RungNode[] {
  return nodes.map((n) =>
    n.type === "parallel" && n.id === groupId
      ? { ...n, branches: [...n.branches, [seed]] }
      : n
  );
}

/** Append an element to a specific branch of a parallel group. */
export function addToParallelBranch(
  nodes: RungNode[],
  groupId: string,
  branchIdx: number,
  el: LadderElement
): RungNode[] {
  return nodes.map((n) =>
    n.type === "parallel" && n.id === groupId
      ? { ...n, branches: n.branches.map((b, i) => (i === branchIdx ? [...b, el] : b)) }
      : n
  );
}

/** Remove one branch of a parallel group; drop the group if it becomes empty. */
export function removeParallelBranch(
  nodes: RungNode[],
  groupId: string,
  branchIdx: number
): RungNode[] {
  const out: RungNode[] = [];
  for (const n of nodes) {
    if (n.type === "parallel" && n.id === groupId) {
      const branches = n.branches.filter((_, i) => i !== branchIdx);
      if (branches.length > 0) out.push({ ...n, branches });
    } else {
      out.push(n);
    }
  }
  return out;
}

/** Build a parallel group node. */
export function makeParallelGroup(id: string, branches: LadderElement[][]): ParallelNode {
  return { type: "parallel", id, branches };
}
