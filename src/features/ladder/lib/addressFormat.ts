/**
 * User-facing PLC variable notation:
 *   I0, I0.0     → discrete input
 *   Q1, Q1.0     → coil / output
 *   M2, M2.3     → marker bit (mapped to holding word bit)
 *   R10          → holding word (register)
 *   R1.5         → bit 5 of holding register 1
 */
import type { Address, MemArea } from "../../../shared/lib/types";

export type VarPrefix = "I" | "Q" | "M" | "R";

export interface ParsedVar {
  prefix: VarPrefix;
  index: number;
  bit: number | null;
  address: Address;
  display: string;
}

/** Map UI prefix → memory area */
export function prefixToArea(prefix: VarPrefix): MemArea {
  switch (prefix) {
    case "I":
      return "discrete";
    case "Q":
      return "coil";
    case "M":
    case "R":
      return "holding";
  }
}

export function areaToPrefix(area: MemArea, hasBit: boolean): VarPrefix {
  switch (area) {
    case "discrete":
      return "I";
    case "coil":
      return "Q";
    case "holding":
      return hasBit ? "M" : "R"; // default: bit → M, word → R (user can pick R1.x)
    case "input_reg":
      return "R";
  }
}

export function formatAddress(addr: Address | null | undefined): string {
  if (!addr) return "—";
  const bit = addr.bit;
  switch (addr.area) {
    case "discrete":
      return bit != null ? `I${addr.index}.${bit}` : `I${addr.index}`;
    case "coil":
      return bit != null ? `Q${addr.index}.${bit}` : `Q${addr.index}`;
    case "holding":
      if (bit != null) return `R${addr.index}.${bit}`;
      return `R${addr.index}`;
    case "input_reg":
      if (bit != null) return `IW${addr.index}.${bit}`;
      return `IW${addr.index}`;
  }
}

/**
 * Parse strings like: I0, Q1, M2.3, R10, R1.5, %I0.0, i0
 */
export function parseVarString(raw: string): ParsedVar | null {
  const s = raw.trim().toUpperCase().replace(/^%/, "");
  // I12.3 / Q0 / M1.0 / R5.7 / R12
  const m = s.match(/^(I|Q|M|R|IW|MW)(\d+)(?:\.(\d+))?$/);
  if (!m) return null;
  let prefix = m[1] as string;
  if (prefix === "IW" || prefix === "MW") prefix = "R";
  const p = prefix as VarPrefix;
  if (p !== "I" && p !== "Q" && p !== "M" && p !== "R") return null;
  const index = Number(m[2]);
  if (!Number.isFinite(index) || index < 0 || index > 65535) return null;
  let bit: number | null = m[3] != null ? Number(m[3]) : null;
  if (bit != null && (bit < 0 || bit > 15)) return null;

  // I/Q: bit optional, usually ignored (bit 0 style) — keep for display only
  // M without bit → treat as bit 0 of holding word M (index as bit flat? or word index bit 0)
  // User said M and R1.x — M can be flat marker: M5 → holding 0 bit packing or M as holding index bit 0
  // Convention: M5 = holding word 5 bit 0, M5.3 = holding 5 bit 3
  // R5 = holding word 5 (no bit), R5.3 = bit 3
  if (p === "M" && bit == null) bit = 0;

  const address: Address = {
    area: prefixToArea(p),
    index,
    bit: p === "R" && bit == null ? undefined : bit ?? undefined,
  };
  // clean undefined
  if (address.bit === undefined) delete (address as { bit?: number }).bit;

  const display = formatAddress(address);
  return { prefix: p, index, bit, address, display };
}

export function addressToForm(addr: Address): {
  prefix: VarPrefix;
  index: number;
  bit: number | null;
  useBit: boolean;
} {
  const prefix = areaToPrefix(addr.area, addr.bit != null);
  // Prefer R when holding with bit was stored as R1.x
  let p = prefix;
  if (addr.area === "holding") {
    p = addr.bit != null ? "R" : "R"; // both R; M is alias for bit access UX
  }
  if (addr.area === "discrete") p = "I";
  if (addr.area === "coil") p = "Q";
  return {
    prefix: p,
    index: addr.index,
    bit: addr.bit ?? null,
    useBit: addr.bit != null || p === "M",
  };
}

export function formToAddress(
  prefix: VarPrefix,
  index: number,
  bit: number | null,
  forceBit: boolean
): Address {
  const area = prefixToArea(prefix);
  const useBit =
    forceBit || prefix === "M" || (prefix === "R" && bit != null);
  const b = useBit ? (bit ?? 0) : null;
  const addr: Address = { area, index };
  if (b != null && (area === "holding" || area === "input_reg")) {
    addr.bit = Math.max(0, Math.min(15, b));
  }
  return addr;
}

export const ADDRESS_HELP_MD = `
# PLC variable addressing

## Prefixes

| Prefix | Meaning | Examples |
|---------|-----------|-----------|
| **I** | Discrete input | \`I0\`, \`I12\` |
| **Q** | Output / coil | \`Q0\`, \`Q3\` |
| **M** | Memory bit | \`M0\`, \`M2.5\` |
| **R** | Word register (Register / MW) | \`R10\`, \`R1.3\` |

## Register bit syntax

\`\`\`
R<register_number>.<bit_number>
\`\`\`

- Bit number: **0–15** (16-bit word)
- Example: \`R1.0\` = bit 0 of register R1, \`R1.15\` = bit 15

## Simulator memory mapping

| Prefix | Memory area |
|---------|----------------|
| I | Discrete Inputs (1x / I) |
| Q | Coils (0x / Q) |
| M | Holding (4x) — bit access |
| R | Holding (4x) — word or \`.x\` bit |

## Ladder elements

- **Contacts / coils** — usually **I**, **Q**, **M**, **R.n.x**
- **P edge (rising)** — one-scan pulse on 0→1
- **N edge (falling)** — one-scan pulse on 1→0
- **SET (S)** — sets a bit to 1 and holds it until RESET
- **RESET (R)** — clears a bit to 0
- **TON / CTU / Math** — **R** registers (words), done outputs on **Q** or **M**
- **MOVE / CMP** — **R** word operands

## Examples

| Syntax | Description |
|-------|------|
| \`I0\` | Start on input 0 |
| \`Q0\` | Output coil 0 |
| \`M5.2\` | Marker bit 2 in R5 |
| \`R20\` | MW20 word (setpoint) |
| \`R1.7\` | Bit 7 in register 1 |

## Tips

1. Click a ladder element to open this window.
2. Choose a prefix (I/Q/M/R), number, and optional bit.
3. You can also type an address manually in the **Quick address** field (e.g. \`R1.3\`).
4. **Apply** saves the element and updates the program (Compile & Load recommended).
`.trim();
