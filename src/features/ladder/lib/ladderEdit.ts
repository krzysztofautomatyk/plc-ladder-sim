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

/**
 * Move a top-level node (element or parallel group) one step left (dir=-1) or right (dir=+1).
 * Coils stay after non-coils when possible: swap only within the same "rail" group
 * (non-coils among themselves, coils among themselves).
 */
export function moveNodeById(nodes: RungNode[], id: string, dir: -1 | 1): RungNode[] {
  const i = nodes.findIndex((n) => n.id === id);
  if (i < 0) return nodes;
  const j = i + dir;
  if (j < 0 || j >= nodes.length) return nodes;
  // Keep coils on the right: don't swap a non-coil past a coil or vice versa.
  const a = nodes[i];
  const b = nodes[j];
  if (isCoilNode(a) !== isCoilNode(b)) return nodes;
  const next = [...nodes];
  next[i] = b;
  next[j] = a;
  return next;
}

/** Move an element inside a parallel group's branch left/right. */
export function moveInParallelBranch(
  nodes: RungNode[],
  groupId: string,
  branchIdx: number,
  elementId: string,
  dir: -1 | 1
): RungNode[] {
  return nodes.map((n) => {
    if (n.type !== "parallel" || n.id !== groupId) return n;
    return {
      ...n,
      branches: n.branches.map((b, bi) => {
        if (bi !== branchIdx) return b;
        const i = b.findIndex((e) => e.id === elementId);
        if (i < 0) return b;
        const j = i + dir;
        if (j < 0 || j >= b.length) return b;
        const next = [...b];
        next[i] = b[j];
        next[j] = b[i];
        return next;
      }),
    };
  });
}

/** Move an element inside a leading OR-branch array left/right. */
export function moveInOrBranch(
  branches: LadderElement[][],
  branchIdx: number,
  elementId: string,
  dir: -1 | 1
): LadderElement[][] {
  return branches.map((b, bi) => {
    if (bi !== branchIdx) return b;
    const i = b.findIndex((e) => e.id === elementId);
    if (i < 0) return b;
    const j = i + dir;
    if (j < 0 || j >= b.length) return b;
    const next = [...b];
    next[i] = b[j];
    next[j] = b[i];
    return next;
  });
}

/**
 * Move an element up (dir=-1) or down (dir=+1) between OR / parallel branches.
 * Appends to the target branch; removes empty source branch.
 */
export function moveBetweenOrBranches(
  branches: LadderElement[][],
  fromBranch: number,
  elementId: string,
  dir: -1 | 1
): LadderElement[][] {
  const toBranch = fromBranch + dir;
  if (toBranch < 0 || toBranch >= branches.length) return branches;
  const from = branches[fromBranch];
  const i = from.findIndex((e) => e.id === elementId);
  if (i < 0) return branches;
  const el = from[i];
  const newFrom = from.filter((_, j) => j !== i);
  const newTo = [...branches[toBranch], el];
  const next = branches.map((b, bi) => {
    if (bi === fromBranch) return newFrom;
    if (bi === toBranch) return newTo;
    return b;
  });
  // Drop emptied source branch (compiler rejects empty OR branches).
  return next.filter((b) => b.length > 0);
}

/** Same as moveBetweenOrBranches but inside a parallel group node. */
export function moveBetweenParallelBranches(
  nodes: RungNode[],
  groupId: string,
  fromBranch: number,
  elementId: string,
  dir: -1 | 1
): RungNode[] {
  return nodes.map((n) => {
    if (n.type !== "parallel" || n.id !== groupId) return n;
    const branches = moveBetweenOrBranches(n.branches, fromBranch, elementId, dir);
    if (branches.length === 0) {
      // Group emptied — drop it from the series (caller may re-insert).
      return n;
    }
    return { ...n, branches };
  }).filter((n) => !(n.type === "parallel" && n.id === groupId && n.branches.length === 0));
}

/**
 * Move a main-series element into an OR branch (leading or_branches).
 * Removes from series; appends to the target branch.
 */
export function moveSeriesToOrBranch(
  elements: RungNode[],
  orBranches: LadderElement[][],
  elementId: string,
  branchIdx: number
): { elements: RungNode[]; or_branches: LadderElement[][] } | null {
  if (branchIdx < 0 || branchIdx >= orBranches.length) return null;
  const i = elements.findIndex((n) => n.id === elementId && n.type !== "parallel");
  if (i < 0) return null;
  const node = elements[i];
  if (node.type === "parallel" || isCoilNode(node)) return null;
  const el = node as LadderElement;
  const nextElements = elements.filter((_, j) => j !== i);
  const nextOr = orBranches.map((b, bi) => (bi === branchIdx ? [...b, el] : b));
  return { elements: nextElements, or_branches: nextOr };
}

/**
 * Move an OR-branch element out into the main series.
 * place "start" = first contact after the OR block (visual right of OR).
 * place "end" = last contact before coils.
 */
export function moveOrBranchToSeries(
  elements: RungNode[],
  orBranches: LadderElement[][],
  branchIdx: number,
  elementId: string,
  place: "start" | "end" = "start"
): { elements: RungNode[]; or_branches: LadderElement[][] } | null {
  if (branchIdx < 0 || branchIdx >= orBranches.length) return null;
  const branch = orBranches[branchIdx];
  const i = branch.findIndex((e) => e.id === elementId);
  if (i < 0) return null;
  const el = branch[i];
  const newBranch = branch.filter((_, j) => j !== i);
  let nextOr = orBranches.map((b, bi) => (bi === branchIdx ? newBranch : b));
  nextOr = nextOr.filter((b) => b.length > 0);
  let nextElements: RungNode[];
  if (place === "start") {
    // First contact after the leading OR block (visual right of OR).
    const contacts = elements.filter((n) => !isCoilNode(n));
    const coils = elements.filter(isCoilNode);
    nextElements = [el, ...contacts, ...coils];
  } else {
    nextElements = insertBeforeCoils(elements, el);
  }
  return { elements: nextElements, or_branches: nextOr };
}

/**
 * Smart series move: if neighbor is a parallel group, enter it instead of swapping past it.
 * dir +1 = right, -1 = left.
 */
export function moveSeriesSmart(
  nodes: RungNode[],
  elementId: string,
  dir: -1 | 1,
  parallelBranchIdx = 0
): RungNode[] {
  const i = nodes.findIndex((n) => n.id === elementId);
  if (i < 0) return nodes;
  const node = nodes[i];
  if (node.type === "parallel" || isCoilNode(node)) {
    return moveNodeById(nodes, elementId, dir);
  }
  const j = i + dir;
  if (j < 0 || j >= nodes.length) return nodes;
  const neighbor = nodes[j];
  if (neighbor.type === "parallel") {
    // Enter the parallel group instead of jumping over it.
    const bi = Math.max(0, Math.min(parallelBranchIdx, neighbor.branches.length - 1));
    const el = node as LadderElement;
    const without = nodes.filter((_, k) => k !== i);
    // Re-find group index after removal
    const gi = without.findIndex((n) => n.type === "parallel" && n.id === neighbor.id);
    if (gi < 0) return nodes;
    return without.map((n, idx) => {
      if (idx !== gi || n.type !== "parallel") return n;
      const branches = n.branches.map((b, bii) =>
        bii === bi ? (dir === 1 ? [el, ...b] : [...b, el]) : b
      );
      return { ...n, branches };
    });
  }
  return moveNodeById(nodes, elementId, dir);
}

/** Exit a parallel-group element into the series before/after the group. */
export function moveParallelToSeries(
  nodes: RungNode[],
  groupId: string,
  branchIdx: number,
  elementId: string,
  place: "before" | "after"
): RungNode[] {
  const gi = nodes.findIndex((n) => n.type === "parallel" && n.id === groupId);
  if (gi < 0) return nodes;
  const group = nodes[gi];
  if (group.type !== "parallel") return nodes;
  if (branchIdx < 0 || branchIdx >= group.branches.length) return nodes;
  const branch = group.branches[branchIdx];
  const ei = branch.findIndex((e) => e.id === elementId);
  if (ei < 0) return nodes;
  const el = branch[ei];
  const newBranch = branch.filter((_, j) => j !== ei);
  let newBranches = group.branches.map((b, bi) => (bi === branchIdx ? newBranch : b));
  newBranches = newBranches.filter((b) => b.length > 0);

  const next = [...nodes];
  if (newBranches.length === 0) {
    next.splice(gi, 1);
    const insertAt = place === "before" ? gi : gi;
    next.splice(insertAt, 0, el);
  } else {
    next[gi] = { ...group, branches: newBranches };
    const insertAt = place === "before" ? gi : gi + 1;
    next.splice(insertAt, 0, el);
  }
  // Keep coil rail ordered
  const contacts = next.filter((n) => !isCoilNode(n));
  const coils = next.filter(isCoilNode);
  return [...contacts, ...coils];
}
