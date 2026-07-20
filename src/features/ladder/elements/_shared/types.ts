/**
 * Shared contracts for ladder instruction packages.
 * One folder per IEC instruction: definition.ts + glyph.svelte.
 */
import type { Address, LadderElement } from "../../../../shared/lib/types";
import type { Component } from "svelte";

/** Stroke props for IEC bit-logic SVG glyphs. */
export interface GlyphProps {
  strokeIn: string;
  strokeOut: string;
  strokeBody: string;
  fillCoil?: string;
  sw?: number;
}

/** Unified props passed to every instruction glyph. */
export type ElementRenderProps<T extends LadderElement = LadderElement> = GlyphProps & {
  element: T;
  active?: boolean;
};

export type ElementCategory = "bit_logic" | "timer" | "counter" | "math" | "wire";

export type CellClass = "contact" | "coil" | "edge" | "fb" | "wire";

export type ElementType = LadderElement["type"];

export type AddressFormatter = (a: Address) => string;

export interface ElementDefinition<K extends ElementType = ElementType> {
  kind: K;
  category: ElementCategory;
  label: string;
  shortLabel: string;
  help: string;
  cellClass: CellClass;
  create: () => Extract<LadderElement, { type: K }>;
  topLabel: (el: Extract<LadderElement, { type: K }>, fmt: AddressFormatter) => string;
  bottomLabel: (el: Extract<LadderElement, { type: K }>, fmt: AddressFormatter) => string;
}

export type ElementGlyph = Component<ElementRenderProps>;

export interface RegistryEntry {
  def: ElementDefinition;
  Glyph: ElementGlyph;
}

export const CATEGORY_TITLES: Record<ElementCategory, string> = {
  bit_logic: "Bit logic",
  timer: "Timers",
  counter: "Counters",
  math: "Math / move / cmp",
  wire: "Wire",
};

/** Palette order of categories. */
export const CATEGORY_ORDER: ElementCategory[] = [
  "bit_logic",
  "timer",
  "counter",
  "math",
  "wire",
];
