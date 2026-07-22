/** Online monitor stroke palette for ladder glyphs. */

export const COL_GREEN = "#00a651";
export const COL_BLUE = "#1565c0";
export const COL_OFF = "#1a1a1a";

export interface GlyphStrokes {
  strokeIn: string;
  strokeOut: string;
  strokeBody: string;
  fillCoil: string;
  sw: number;
}

export function resolveStrokes(opts: {
  active: boolean;
  powerIn: boolean;
  energized: boolean;
  isCoil: boolean;
}): GlyphStrokes {
  const { active, powerIn, energized, isCoil } = opts;
  const inLit = powerIn || active;
  // Contacts: only green power-flow (never blue stubs/poles).
  // Coils: blue when the output bit is ON in the process image.
  return {
    strokeIn: inLit ? COL_GREEN : COL_OFF,
    strokeOut: isCoil
      ? energized
        ? COL_BLUE
        : active
          ? COL_GREEN
          : COL_OFF
      : active
        ? COL_GREEN
        : COL_OFF,
    strokeBody: isCoil
      ? energized
        ? COL_BLUE
        : active
          ? COL_GREEN
          : COL_OFF
      : active
        ? COL_GREEN
        : COL_OFF,
    fillCoil: energized && isCoil ? "rgba(21, 101, 192, 0.18)" : "none",
    sw: 2,
  };
}
