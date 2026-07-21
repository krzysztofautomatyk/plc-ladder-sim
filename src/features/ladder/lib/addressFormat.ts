/**
 * Canonical PLC variable notation:
 *   I / Q / M / MR / R / IW  — process image
 *   TV0 / CV0               — timer/counter current value (word) → holding bank
 *   T0 / C0                 — timer/counter done bit (word Q in bank)
 *
 * TV/CV compile as holding addresses so MOVE/math can copy T/C → R.
 * Pure bit areas (I/Q/M) reject .bit syntax.
 */
import type { Address, MemArea } from "../../../shared/lib/types";

/** Must match backend `TIMER_HR_BASE` / `COUNTER_HR_BASE`. */
export const TIMER_HR_BASE = 2048;
export const COUNTER_HR_BASE = 3072;

export type VarPrefix = "I" | "Q" | "M" | "R" | "MR" | "IW" | "TV" | "CV" | "T" | "C";

export interface ParsedVar {
  prefix: VarPrefix;
  index: number;
  bit: number | null;
  address: Address;
  display: string;
}

export function timerValueAddress(n: number): Address {
  return { area: "holding", index: TIMER_HR_BASE + 2 * n };
}

export function timerDoneAddress(n: number): Address {
  return { area: "holding", index: TIMER_HR_BASE + 2 * n + 1 };
}

export function counterValueAddress(n: number): Address {
  return { area: "holding", index: COUNTER_HR_BASE + 2 * n };
}

export function counterDoneAddress(n: number): Address {
  return { area: "holding", index: COUNTER_HR_BASE + 2 * n + 1 };
}

/** Map UI prefix → memory area (TV/CV/T/C map via dedicated helpers in parse). */
export function prefixToArea(prefix: VarPrefix): MemArea {
  switch (prefix) {
    case "I":
      return "discrete";
    case "Q":
      return "coil";
    case "M":
      return "memory_bit";
    case "MR":
      return "memory_word";
    case "R":
    case "TV":
    case "CV":
    case "T":
    case "C":
      return "holding";
    case "IW":
      return "input_reg";
  }
}

export function areaToPrefix(area: MemArea, _hasBit: boolean): VarPrefix {
  switch (area) {
    case "discrete":
      return "I";
    case "coil":
      return "Q";
    case "memory_bit":
      return "M";
    case "memory_word":
      return "MR";
    case "holding":
      return "R";
    case "input_reg":
      return "IW";
  }
}

/** Pretty-print holding bank addresses as TV/CV/T/C when they land in T/C banks. */
function formatHolding(index: number, bit: number | null | undefined): string {
  if (bit != null) {
    return `R${index}.${bit}`;
  }
  // Timer value bank: even offsets from TIMER_HR_BASE
  if (index >= TIMER_HR_BASE && index < TIMER_HR_BASE + 512) {
    const off = index - TIMER_HR_BASE;
    if (off % 2 === 0) return `TV${off / 2}`;
    return `T${(off - 1) / 2}`;
  }
  if (index >= COUNTER_HR_BASE && index < COUNTER_HR_BASE + 512) {
    const off = index - COUNTER_HR_BASE;
    if (off % 2 === 0) return `CV${off / 2}`;
    return `C${(off - 1) / 2}`;
  }
  return `R${index}`;
}

export function formatAddress(addr: Address | null | undefined): string {
  if (!addr) return "—";
  const bit = addr.bit;
  switch (addr.area) {
    case "discrete":
      return `I${addr.index}`;
    case "coil":
      return `Q${addr.index}`;
    case "memory_bit":
      return `M${addr.index}`;
    case "memory_word":
      return bit != null ? `MR${addr.index}.${bit}` : `MR${addr.index}`;
    case "holding":
      return formatHolding(addr.index, bit);
    case "input_reg":
      return bit != null ? `IW${addr.index}.${bit}` : `IW${addr.index}`;
  }
}

/**
 * Parse: I0, Q1, M2, MR5, R10, R1.5, IW4, MW20, TV0, CV3, T0, C1, %I0
 */
export function parseVarString(raw: string): ParsedVar | null {
  const s = raw.trim().toUpperCase().replace(/^%/, "");
  // Longest tokens first: TV/CV/MR/IW/MW before T/C/M/I/R.
  const m = s.match(/^(TV|CV|MR|IW|MW|T|C|I|Q|M|R)(\d+)(?:\.(\d+))?$/);
  if (!m) return null;

  let token = m[1] as string;
  if (token === "MW") token = "R";

  const index = Number(m[2]);
  if (!Number.isFinite(index) || index < 0 || index > 65535) return null;

  let bit: number | null = m[3] != null ? Number(m[3]) : null;
  if (bit != null && (bit < 0 || bit > 15)) return null;

  // Timer / counter aliases → holding banks (engine-readable for MOVE).
  if (token === "TV") {
    if (bit != null || index > 255) return null;
    const address = timerValueAddress(index);
    return { prefix: "TV", index, bit: null, address, display: `TV${index}` };
  }
  if (token === "CV") {
    if (bit != null || index > 255) return null;
    const address = counterValueAddress(index);
    return { prefix: "CV", index, bit: null, address, display: `CV${index}` };
  }
  if (token === "T") {
    if (bit != null || index > 255) return null;
    const address = timerDoneAddress(index);
    return { prefix: "T", index, bit: null, address, display: `T${index}` };
  }
  if (token === "C") {
    if (bit != null || index > 255) return null;
    const address = counterDoneAddress(index);
    return { prefix: "C", index, bit: null, address, display: `C${index}` };
  }

  const p = token as VarPrefix;
  if (p !== "I" && p !== "Q" && p !== "M" && p !== "MR" && p !== "R" && p !== "IW") {
    return null;
  }

  // Pure bit areas — no sub-bit addressing.
  if (p === "I" || p === "Q" || p === "M") {
    if (bit != null) return null;
  }

  const address: Address = { area: prefixToArea(p), index };
  if (bit != null) address.bit = bit;

  const display = formatAddress(address);
  return { prefix: p, index, bit, address, display };
}

export function addressToForm(addr: Address): {
  prefix: VarPrefix;
  index: number;
  bit: number | null;
  useBit: boolean;
} {
  // Prefer T/C aliases when holding index is in a bank.
  if (addr.area === "holding" && addr.bit == null) {
    const i = addr.index;
    if (i >= TIMER_HR_BASE && i < TIMER_HR_BASE + 512) {
      const off = i - TIMER_HR_BASE;
      if (off % 2 === 0) {
        return { prefix: "TV", index: off / 2, bit: null, useBit: false };
      }
      return { prefix: "T", index: (off - 1) / 2, bit: null, useBit: false };
    }
    if (i >= COUNTER_HR_BASE && i < COUNTER_HR_BASE + 512) {
      const off = i - COUNTER_HR_BASE;
      if (off % 2 === 0) {
        return { prefix: "CV", index: off / 2, bit: null, useBit: false };
      }
      return { prefix: "C", index: (off - 1) / 2, bit: null, useBit: false };
    }
  }
  const prefix = areaToPrefix(addr.area, addr.bit != null);
  return {
    prefix,
    index: addr.index,
    bit: addr.bit ?? null,
    useBit: addr.bit != null,
  };
}

export function formToAddress(
  prefix: VarPrefix,
  index: number,
  bit: number | null,
  forceBit: boolean
): Address {
  if (prefix === "TV") return timerValueAddress(index);
  if (prefix === "CV") return counterValueAddress(index);
  if (prefix === "T") return timerDoneAddress(index);
  if (prefix === "C") return counterDoneAddress(index);

  const area = prefixToArea(prefix);
  const supportsBit = area === "holding" || area === "input_reg" || area === "memory_word";
  const useBit = supportsBit && (forceBit || bit != null);
  const addr: Address = { area, index };
  if (useBit) {
    addr.bit = Math.max(0, Math.min(15, bit ?? 0));
  }
  return addr;
}

export const ADDRESS_HELP_MD = `
# PLC variable addressing

## Process image

| Prefix | Meaning | Examples |
|---------|-----------|-----------|
| **I** | Discrete input | \`I0\` |
| **Q** | Output / coil | \`Q0\` |
| **M** | Marker bit | \`M0\` |
| **MR** | Internal register | \`MR0\`, \`MR1.3\` |
| **R** | Holding word | \`R10\`, \`R1.3\` |
| **IW** | Input register | \`IW0\` |

## Timers / counters (live values you can MOVE)

| Prefix | Meaning | Example use |
|---------|-----------|-------------|
| **TV**n | Timer elapsed (ms) word | \`MOVE TV0 → R20\` |
| **T**n | Timer done bit (Q) | contact on \`T0\` |
| **CV**n | Counter current value | \`MOVE CV0 → R21\` |
| **C**n | Counter done bit (Q) | contact on \`C0\` |

These map to engine banks (R2048+ for T, R3072+ for C) and always appear live on FB blocks.

## Tips

1. On TON/CTU blocks, **ET/CV** update while RUN.
2. To copy a timer into a register: add **MOVE**, source \`TV0\`, dest \`R10\`.
3. Alias: **MW**n → **R**n.
`.trim();
