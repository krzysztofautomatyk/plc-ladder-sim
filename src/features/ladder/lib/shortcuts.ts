import type { PaletteKind } from "../../../shared/lib/types";

/**
 * Keyboard shortcuts for the ladder instruction toolbar.
 * Shared by the toolbar (to render kbd hints) and the editor (to handle keys),
 * so the mapping never drifts between the two.
 */
export const INSTRUCTION_SHORTCUTS: Record<string, PaletteKind> = {
  "1": "contact_no",
  "2": "contact_nc",
  "3": "contact_rising",
  "4": "contact_falling",
  "5": "coil",
  "6": "coil_negated",
  "7": "coil_set",
  "8": "coil_reset",
  o: "or_branch",
  w: "wire",
  t: "ton",
  c: "ctu",
  m: "math",
};

/** The shortcut key that inserts a given instruction, if any. */
export function shortcutFor(kind: PaletteKind): string | undefined {
  for (const [key, k] of Object.entries(INSTRUCTION_SHORTCUTS)) {
    if (k === kind) return key;
  }
  return undefined;
}

/** The instruction bound to a keyboard key (case-insensitive), if any. */
export function kindForKey(key: string): PaletteKind | undefined {
  return INSTRUCTION_SHORTCUTS[key.toLowerCase()];
}
