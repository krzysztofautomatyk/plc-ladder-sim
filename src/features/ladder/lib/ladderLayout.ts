/**
 * TIA-style ladder layout helpers: output coils right-justified,
 * series logic on the left, power-flow flags for online monitor.
 */
import type { LadderElement, RungNode } from "../../../shared/lib/types";

const OUTPUT_TYPES = new Set([
  "coil",
  "coil_negated",
  "coil_set",
  "coil_reset",
]);

/** Elements that sit on the right rail (classic TIA coil column). */
export function isRightRailElement(el: RungNode): boolean {
  return el.type !== "parallel" && OUTPUT_TYPES.has(el.type);
}

export function splitRungElements(elements: RungNode[]): {
  left: RungNode[];
  right: LadderElement[];
} {
  const left: RungNode[] = [];
  const right: LadderElement[] = [];
  for (const el of elements) {
    if (el.type !== "parallel" && OUTPUT_TYPES.has(el.type)) right.push(el);
    else left.push(el);
  }
  return { left, right };
}

/**
 * Power into node i in a series chain.
 * Left rail always feeds powerIn=true for index 0.
 * After that, powerIn = previous node conducted.
 */
export function seriesPowerIn(
  nodes: RungNode[],
  index: number,
  conducts: (n: RungNode) => boolean
): boolean {
  if (index <= 0) return true;
  const prev = nodes[index - 1];
  return prev ? conducts(prev) : true;
}

/** Wire segment after node i is hot when that node conducted. */
export function seriesPowerOut(
  nodes: RungNode[],
  index: number,
  conducts: (n: RungNode) => boolean
): boolean {
  const n = nodes[index];
  return n ? conducts(n) : false;
}

export interface RungColumns {
  /** Parallel OR branches evaluated before the series (may be empty). */
  branches: LadderElement[][];
  hasParallel: boolean;
  /** Series nodes (contacts, FBs, inline parallel groups) after the OR merge. */
  series: RungNode[];
  /** Right-rail output coils. */
  coils: LadderElement[];
}

/**
 * Structural columns of a rung for rendering: the parallel OR block, the series
 * logic after the merge, and the right-rail coils. Pure — safe to unit test.
 */
export function rungColumns(rung: {
  elements: RungNode[];
  or_branches?: LadderElement[][];
}): RungColumns {
  const { left, right } = splitRungElements(rung.elements ?? []);
  const branches = rung.or_branches ?? [];
  return {
    branches,
    hasParallel: branches.length > 0,
    series: left,
    coils: right,
  };
}
