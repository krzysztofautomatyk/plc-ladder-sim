/**
 * Ladder instruction library — public API.
 * Rule: new instruction = folder + registry entry.
 */
export type {
  CellClass,
  ElementCategory,
  ElementDefinition,
  ElementGlyph,
  ElementRenderProps,
  ElementType,
  GlyphProps,
  RegistryEntry,
} from "./_shared/types";

export {
  CATEGORY_ORDER,
  CATEGORY_TITLES,
} from "./_shared/types";

export {
  ELEMENT_REGISTRY,
  createElement,
  getDefinition,
  getRegistryEntry,
  isBitElement,
  isCoilType,
  isCounterType,
  isFbType,
  isTimerType,
  paletteGroups,
  type PaletteGroup,
} from "./_shared/registry";

export { newId } from "./_shared/id";
