/**
 * World-class ladder element movement — single source of truth.
 *
 * Visual layout (left → right on canvas):
 *   [ leading OR block ] → [ series contacts / inline parallel ] → [ coils ]
 *
 * Rules (TIA-like, match what the user SEES):
 * 1. ‹ / › move along one horizontal rail (one OR/∥ branch row).
 * 2. ▲ / ▼ change branch row, keeping column when possible.
 * 3. Never jump over an inline parallel group (enter it instead of swapping past).
 * 4. Exit a parallel/OR block only at the RIGHT edge of the WHOLE block
 *    (max branch width), not merely "last element of a short branch".
 * 5. Enter leading OR only from the true left edge of the series array
 *    (after checking an immediate parallel neighbor).
 * 6. Coils stay on the right rail; ▲/▼ reorder the coil stack.
 */
import type { LadderElement, ParallelNode, Rung, RungNode } from "../../../shared/lib/types";
import { isCoilNode } from "./ladderEdit";

export type MoveDir = "left" | "right" | "up" | "down";

export type ElementLoc =
  | { kind: "series"; index: number }
  | { kind: "or"; branch: number; index: number }
  | { kind: "par"; groupId: string; groupIndex: number; branch: number; index: number };

export type MoveOpts = {
  /**
   * Leading OR enter mode:
   * - number ≥ 0 → APPEND to that branch (AND series, right end of the row)
   * - null / undefined → NEW branch at the bottom (OR alternative)
   */
  orBranchHint?: number | null;
  /**
   * Inline parallel enter mode (same rules as orBranchHint).
   * Only applied when parGroupHint matches the group being entered (or is null).
   */
  parBranchHint?: number | null;
  /** Optional group id — if set, parBranchHint only applies to this group. */
  parGroupHint?: string | null;
};

function isParallel(n: RungNode): n is ParallelNode {
  return n.type === "parallel";
}

function asElement(n: RungNode): LadderElement | null {
  if (n.type === "parallel") return null;
  return n as LadderElement;
}

function maxBranchWidth(branches: LadderElement[][]): number {
  let m = 0;
  for (const b of branches) if (b.length > m) m = b.length;
  return m;
}

function orderContactsThenCoils(nodes: RungNode[]): RungNode[] {
  return [...nodes.filter((n) => !isCoilNode(n)), ...nodes.filter(isCoilNode)];
}

function deepCloneLocResult(
  elements: RungNode[],
  orBranches: LadderElement[][]
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  return {
    elements: elements.map((n) =>
      isParallel(n) ? { ...n, branches: n.branches.map((b) => [...b]) } : { ...n }
    ),
    or_branches: orBranches.map((b) => [...b]),
  };
}

function unchanged(
  elements: RungNode[],
  orBranches: LadderElement[][]
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  return { elements, or_branches: orBranches };
}

// ── Locate ────────────────────────────────────────────────────────────────

/** Locate an element (or top-level parallel group id). */
export function findInRung(
  elements: RungNode[],
  orBranches: LadderElement[][],
  id: string
): ElementLoc | null {
  for (let bi = 0; bi < orBranches.length; bi++) {
    const ei = orBranches[bi].findIndex((e) => e.id === id);
    if (ei >= 0) return { kind: "or", branch: bi, index: ei };
  }
  for (let i = 0; i < elements.length; i++) {
    const n = elements[i];
    if (n.id === id) return { kind: "series", index: i };
    if (isParallel(n)) {
      for (let bi = 0; bi < n.branches.length; bi++) {
        const ei = n.branches[bi].findIndex((e) => e.id === id);
        if (ei >= 0)
          return {
            kind: "par",
            groupId: n.id,
            groupIndex: i,
            branch: bi,
            index: ei,
          };
      }
    }
  }
  return null;
}

// ── Public API ────────────────────────────────────────────────────────────

export function canMoveInRung(
  elements: RungNode[],
  orBranches: LadderElement[][],
  id: string,
  dir: MoveDir,
  opts?: MoveOpts
): boolean {
  const next = moveInRung(elements, orBranches, id, dir, opts);
  if (!next) return false;
  return (
    JSON.stringify(next.elements) !== JSON.stringify(elements) ||
    JSON.stringify(next.or_branches) !== JSON.stringify(orBranches)
  );
}

/**
 * Apply a visual move. Returns null if the element is not found;
 * returns same references if the move is a no-op at the boundary.
 */
export function moveInRung(
  elements: RungNode[],
  orBranches: LadderElement[][],
  id: string,
  dir: MoveDir,
  opts?: MoveOpts
): { elements: RungNode[]; or_branches: LadderElement[][] } | null {
  const loc = findInRung(elements, orBranches, id);
  if (!loc) return null;

  switch (loc.kind) {
    case "or":
      return moveFromOr(elements, orBranches, loc, dir);
    case "series":
      return moveFromSeries(elements, orBranches, loc, dir, opts);
    case "par":
      return moveFromPar(elements, orBranches, loc, dir);
  }
}

// ── Leading OR ────────────────────────────────────────────────────────────

function moveFromOr(
  elements: RungNode[],
  orBranches: LadderElement[][],
  loc: Extract<ElementLoc, { kind: "or" }>,
  dir: MoveDir
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  const { branch: bi, index: ei } = loc;
  const branch = orBranches[bi];
  if (!branch || !branch[ei]) return unchanged(elements, orBranches);

  if (dir === "left") {
    if (ei <= 0) return unchanged(elements, orBranches);
    return {
      elements,
      or_branches: swapInBranch(orBranches, bi, ei, ei - 1),
    };
  }

  if (dir === "right") {
    if (ei < branch.length - 1) {
      return {
        elements,
        or_branches: swapInBranch(orBranches, bi, ei, ei + 1),
      };
    }
    // Last in THIS branch — only exit if at the right edge of the WHOLE OR block.
    // Short branches that are narrower than siblings stay put (no "jump past OR").
    const width = maxBranchWidth(orBranches);
    if (ei < width - 1) {
      return unchanged(elements, orBranches);
    }
    return exitOrToSeries(elements, orBranches, bi, ei);
  }

  if (dir === "up" || dir === "down") {
    const d = dir === "up" ? -1 : 1;
    return {
      elements,
      or_branches: moveVert(orBranches, bi, ei, d),
    };
  }

  return unchanged(elements, orBranches);
}

function exitOrToSeries(
  elements: RungNode[],
  orBranches: LadderElement[][],
  bi: number,
  ei: number
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  const el = orBranches[bi][ei];
  const newBranch = orBranches[bi].filter((_, i) => i !== ei);
  let nextOr = orBranches.map((b, i) => (i === bi ? newBranch : b));
  nextOr = nextOr.filter((b) => b.length > 0);
  // Place at start of contact rail (immediately after OR visually)
  const contacts = elements.filter((n) => !isCoilNode(n));
  const coils = elements.filter(isCoilNode);
  return { elements: [el, ...contacts, ...coils], or_branches: nextOr };
}

// ── Main series ───────────────────────────────────────────────────────────

function moveFromSeries(
  elements: RungNode[],
  orBranches: LadderElement[][],
  loc: Extract<ElementLoc, { kind: "series" }>,
  dir: MoveDir,
  opts?: MoveOpts
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  const i = loc.index;
  const node = elements[i];
  if (!node) return unchanged(elements, orBranches);
  // Preserve null = "no selection → new bottom branch". Do NOT default to 0.
  const orHint = opts?.orBranchHint;
  const parHint = opts?.parBranchHint;
  const parGroup = opts?.parGroupHint;

  // Whole parallel group: only left/right swap with non-group logic neighbors
  if (isParallel(node)) {
    if (dir === "up" || dir === "down") return unchanged(elements, orBranches);
    const d = dir === "left" ? -1 : 1;
    const j = i + d;
    if (j < 0 || j >= elements.length) return unchanged(elements, orBranches);
    // Never swap a parallel group past a coil rail boundary incorrectly
    return {
      elements: swapSeries(elements, i, j),
      or_branches: orBranches,
    };
  }

  // Coils: L/R among coils; U/D reorder coil stack (visual vertical)
  if (isCoilNode(node)) {
    return moveCoil(elements, orBranches, i, dir);
  }

  // Contact / FB in series
  if (dir === "left") {
    // 1) Immediate left neighbor first (inline parallel wins over leading OR)
    if (i > 0) {
      const prev = elements[i - 1];
      if (prev && isParallel(prev)) {
        return enterParallel(
          elements,
          orBranches,
          i,
          prev.id,
          "end",
          parHint,
          parGroup
        );
      }
      return {
        elements: swapSeries(elements, i, i - 1),
        or_branches: orBranches,
      };
    }
    // 2) True left edge of series → enter leading OR if present
    if (orBranches.length > 0) {
      return enterLeadingOr(elements, orBranches, i, orHint);
    }
    return unchanged(elements, orBranches);
  }

  if (dir === "right") {
    if (i < elements.length - 1) {
      const next = elements[i + 1];
      if (next && isParallel(next)) {
        // Enter parallel — never jump over the whole OR block
        return enterParallel(
          elements,
          orBranches,
          i,
          next.id,
          "start",
          parHint,
          parGroup
        );
      }
      return {
        elements: swapSeries(elements, i, i + 1),
        or_branches: orBranches,
      };
    }
    return unchanged(elements, orBranches);
  }

  // up/down on series contacts: no-op (only OR/∥/coils use vertical)
  return unchanged(elements, orBranches);
}

function moveCoil(
  elements: RungNode[],
  orBranches: LadderElement[][],
  i: number,
  dir: MoveDir
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  const coilIndices = elements
    .map((n, idx) => ({ n, idx }))
    .filter(({ n }) => isCoilNode(n))
    .map(({ idx }) => idx);
  const pos = coilIndices.indexOf(i);
  if (pos < 0) return unchanged(elements, orBranches);

  if (dir === "left" || dir === "right") {
    // Horizontal: only among coils (already on right rail)
    const d = dir === "left" ? -1 : 1;
    const j = i + d;
    if (j < 0 || j >= elements.length) return unchanged(elements, orBranches);
    if (!isCoilNode(elements[j])) return unchanged(elements, orBranches);
    return {
      elements: swapSeries(elements, i, j),
      or_branches: orBranches,
    };
  }

  // Vertical: reorder coil stack (▲ = earlier in stack / higher on screen)
  const d = dir === "up" ? -1 : 1;
  const otherPos = pos + d;
  if (otherPos < 0 || otherPos >= coilIndices.length) {
    return unchanged(elements, orBranches);
  }
  const j = coilIndices[otherPos];
  return {
    elements: swapSeries(elements, i, j),
    or_branches: orBranches,
  };
}

/**
 * Enter leading OR from series.
 *
 * AND (horizontal): if branchIdx is a valid branch index → append to the RIGHT
 * of that row (series contacts in one alternative).
 * OR  (vertical):   if branchIdx is null/undefined → NEW row at the bottom.
 *
 * UX: click an OR branch (highlight) then ‹ from series → AND into that branch.
 *     no selection → new alternative at bottom.
 */
function enterLeadingOr(
  elements: RungNode[],
  orBranches: LadderElement[][],
  seriesIndex: number,
  branchIdx: number | null | undefined
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  const node = elements[seriesIndex];
  const el = asElement(node);
  if (!el || isCoilNode(node)) return unchanged(elements, orBranches);
  if (orBranches.length === 0) return unchanged(elements, orBranches);
  const nextElements = elements.filter((_, j) => j !== seriesIndex);

  const andInto =
    typeof branchIdx === "number" &&
    branchIdx >= 0 &&
    branchIdx < orBranches.length;

  if (andInto) {
    // AND: dock on the right of the selected branch
    const nextOr = orBranches.map((b, i) =>
      i === branchIdx ? [...b, el] : b
    );
    return { elements: nextElements, or_branches: nextOr };
  }

  // OR: new alternative at the bottom
  return { elements: nextElements, or_branches: [...orBranches, [el]] };
}

/**
 * Enter inline parallel from series — same AND/OR rules as leading OR.
 * AND only when branchHint is valid (and optional groupHint matches).
 */
function enterParallel(
  elements: RungNode[],
  orBranches: LadderElement[][],
  seriesIndex: number,
  groupId: string,
  _side: "start" | "end",
  branchHint: number | null | undefined = null,
  groupHint: string | null | undefined = null
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  const node = elements[seriesIndex];
  const el = asElement(node);
  if (!el || isCoilNode(node)) return unchanged(elements, orBranches);
  const without = elements.filter((_, j) => j !== seriesIndex);
  const gi = without.findIndex((n) => isParallel(n) && n.id === groupId);
  if (gi < 0) return unchanged(elements, orBranches);
  const group = without[gi] as ParallelNode;
  if (group.branches.length === 0) return unchanged(elements, orBranches);

  const hintOk =
    (groupHint == null || groupHint === groupId) &&
    typeof branchHint === "number" &&
    branchHint >= 0 &&
    branchHint < group.branches.length;

  const nextElements = without.map((n, idx) => {
    if (idx !== gi || !isParallel(n)) return n;
    if (hintOk) {
      // AND into selected ∥ branch (right end)
      return {
        ...n,
        branches: n.branches.map((b, bii) =>
          bii === branchHint ? [...b, el] : b
        ),
      };
    }
    // OR: new bottom branch
    return { ...n, branches: [...n.branches, [el]] };
  });
  return { elements: nextElements, or_branches: orBranches };
}

// ── Inline parallel ──────────────────────────────────────────────────────

function moveFromPar(
  elements: RungNode[],
  orBranches: LadderElement[][],
  loc: Extract<ElementLoc, { kind: "par" }>,
  dir: MoveDir
): { elements: RungNode[]; or_branches: LadderElement[][] } {
  const { groupId, groupIndex, branch: bi, index: ei } = loc;
  const group = elements[groupIndex];
  if (!isParallel(group)) return unchanged(elements, orBranches);
  const branch = group.branches[bi];
  if (!branch || !branch[ei]) return unchanged(elements, orBranches);
  const width = maxBranchWidth(group.branches);

  if (dir === "left") {
    if (ei > 0) {
      return {
        elements: swapInParallel(elements, groupId, bi, ei, ei - 1),
        or_branches: orBranches,
      };
    }
    // First in branch → exit BEFORE the parallel group
    return {
      elements: exitParallel(elements, groupId, bi, ei, "before"),
      or_branches: orBranches,
    };
  }

  if (dir === "right") {
    if (ei < branch.length - 1) {
      return {
        elements: swapInParallel(elements, groupId, bi, ei, ei + 1),
        or_branches: orBranches,
      };
    }
    // Last in THIS branch — only exit if at right edge of WHOLE group width.
    // Prevents "› jumps past longer sibling branches".
    if (ei < width - 1) {
      return unchanged(elements, orBranches);
    }
    return {
      elements: exitParallel(elements, groupId, bi, ei, "after"),
      or_branches: orBranches,
    };
  }

  if (dir === "up" || dir === "down") {
    const d = dir === "up" ? -1 : 1;
    const nextBranches = moveVert(group.branches, bi, ei, d);
    if (nextBranches.length === 0) {
      const without = elements.filter((n) => n.id !== groupId);
      const insertAt = Math.min(groupIndex, without.length);
      const next = [...without];
      next.splice(insertAt, 0, branch[ei]);
      return { elements: orderContactsThenCoils(next), or_branches: orBranches };
    }
    return {
      elements: elements.map((n) =>
        isParallel(n) && n.id === groupId ? { ...n, branches: nextBranches } : n
      ),
      or_branches: orBranches,
    };
  }

  return unchanged(elements, orBranches);
}

// ── helpers ──────────────────────────────────────────────────────────────

function swapInBranch(
  branches: LadderElement[][],
  bi: number,
  i: number,
  j: number
): LadderElement[][] {
  return branches.map((b, bii) => {
    if (bii !== bi) return b;
    if (j < 0 || j >= b.length) return b;
    const next = [...b];
    next[i] = b[j];
    next[j] = b[i];
    return next;
  });
}

function swapSeries(nodes: RungNode[], i: number, j: number): RungNode[] {
  if (j < 0 || j >= nodes.length) return nodes;
  const a = nodes[i];
  const b = nodes[j];
  // Never swap contact/FB with coil (coils stay on right rail)
  if (!isParallel(a) && !isParallel(b) && isCoilNode(a) !== isCoilNode(b)) {
    return nodes;
  }
  // Never swap a contact past a parallel by accident — caller should enterParallel.
  // Whole-group moves are allowed (both sides may involve parallel).
  const next = [...nodes];
  next[i] = b;
  next[j] = a;
  return next;
}

function swapInParallel(
  nodes: RungNode[],
  groupId: string,
  bi: number,
  i: number,
  j: number
): RungNode[] {
  return nodes.map((n) => {
    if (!isParallel(n) || n.id !== groupId) return n;
    return {
      ...n,
      branches: n.branches.map((b, bii) => {
        if (bii !== bi) return b;
        if (j < 0 || j >= b.length) return b;
        const next = [...b];
        next[i] = b[j];
        next[j] = b[i];
        return next;
      }),
    };
  });
}

/** Vertical move: keep column when possible. */
function moveVert(
  branches: LadderElement[][],
  fromBi: number,
  fromEi: number,
  dir: -1 | 1
): LadderElement[][] {
  const toBi = fromBi + dir;
  if (toBi < 0 || toBi >= branches.length) return branches;
  const from = branches[fromBi];
  const el = from[fromEi];
  const newFrom = from.filter((_, j) => j !== fromEi);
  const to = branches[toBi];
  const insertAt = Math.min(fromEi, to.length);
  const newTo = [...to.slice(0, insertAt), el, ...to.slice(insertAt)];
  const next = branches.map((b, bi) => {
    if (bi === fromBi) return newFrom;
    if (bi === toBi) return newTo;
    return b;
  });
  return next.filter((b) => b.length > 0);
}

function exitParallel(
  nodes: RungNode[],
  groupId: string,
  bi: number,
  ei: number,
  place: "before" | "after"
): RungNode[] {
  const gi = nodes.findIndex((n) => isParallel(n) && n.id === groupId);
  if (gi < 0) return nodes;
  const group = nodes[gi] as ParallelNode;
  const el = group.branches[bi][ei];
  const newBranch = group.branches[bi].filter((_, j) => j !== ei);
  let newBranches = group.branches.map((b, bii) => (bii === bi ? newBranch : b));
  newBranches = newBranches.filter((b) => b.length > 0);

  const next = [...nodes];
  if (newBranches.length === 0) {
    next.splice(gi, 1);
    next.splice(gi, 0, el);
  } else {
    next[gi] = { ...group, branches: newBranches };
    next.splice(place === "before" ? gi : gi + 1, 0, el);
  }
  return orderContactsThenCoils(next);
}

/** Convenience: apply move to a full Rung. */
export function moveInRungModel(rung: Rung, id: string, dir: MoveDir, opts?: MoveOpts): Rung {
  const result = moveInRung(rung.elements ?? [], rung.or_branches ?? [], id, dir, opts);
  if (!result) return rung;
  return {
    ...rung,
    elements: result.elements,
    or_branches: result.or_branches,
  };
}

/** Describe a move for status-bar feedback (PL/EN short). */
export function describeMove(
  before: { elements: RungNode[]; or_branches: LadderElement[][] },
  after: { elements: RungNode[]; or_branches: LadderElement[][] },
  id: string,
  dir: MoveDir
): string {
  const a = findInRung(before.elements, before.or_branches, id);
  const b = findInRung(after.elements, after.or_branches, id);
  if (!a || !b) return `Move ${dir}`;
  if (a.kind !== b.kind) {
    if (b.kind === "or") return "→ entered leading OR";
    if (b.kind === "par") return "→ entered parallel ∥";
    if (a.kind === "or") return "→ exited OR to series";
    if (a.kind === "par") return "→ exited parallel ∥ to series";
  }
  return `Moved ${dir}`;
}

// Keep deepClone helper available for tests that need isolation
export { deepCloneLocResult as cloneRungParts };
