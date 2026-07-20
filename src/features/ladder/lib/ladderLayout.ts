/**
 * TIA-style ladder layout helpers: output coils right-justified,
 * series logic on the left, power-flow flags for online monitor.
 */
import type { LadderElement } from "../../../shared/lib/types";

const OUTPUT_TYPES = new Set([
  "coil",
  "coil_negated",
  "coil_set",
  "coil_reset",
]);

/** Elements that sit on the right rail (classic TIA coil column). */
export function isRightRailElement(el: LadderElement): boolean {
  return OUTPUT_TYPES.has(el.type);
}

export function splitRungElements(elements: LadderElement[]): {
  left: LadderElement[];
  right: LadderElement[];
} {
  const left: LadderElement[] = [];
  const right: LadderElement[] = [];
  for (const el of elements) {
    if (isRightRailElement(el)) right.push(el);
    else left.push(el);
  }
  return { left, right };
}

/**
 * Power into element i in a series chain.
 * Left rail always feeds powerIn=true for index 0.
 * After that, powerIn = previous element was "active" (conducted).
 */
export function seriesPowerIn(
  elements: LadderElement[],
  index: number,
  isActive: (id: string) => boolean
): boolean {
  if (index <= 0) return true;
  const prev = elements[index - 1];
  return prev ? isActive(prev.id) : true;
}

/** Wire segment after element i is hot when that element conducted. */
export function seriesPowerOut(
  elements: LadderElement[],
  index: number,
  isActive: (id: string) => boolean
): boolean {
  const el = elements[index];
  return el ? isActive(el.id) : false;
}
