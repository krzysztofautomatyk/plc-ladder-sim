/**
 * User-facing PLC variable notation:
 *   I0, I0.0     → discrete input
 *   Q1, Q1.0     → coil / output
 *   M2           → internal marker bit (ladder-only, never on Modbus)
 *   MR5, MR5.3   → internal memory register / bit-in-register (ladder-only, never on Modbus)
 *   R10          → holding word (register, Modbus 4x)
 *   R1.5         → bit 5 of holding register 1
 */
import type { Address, MemArea } from "../../../shared/lib/types";

export type VarPrefix = "I" | "Q" | "M" | "R" | "MR";

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
      return "memory_bit";
    case "MR":
      return "memory_word";
    case "R":
      return "holding";
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
    case "memory_bit":
      return `M${addr.index}`;
    case "memory_word":
      return bit != null ? `MR${addr.index}.${bit}` : `MR${addr.index}`;
    case "holding":
      return bit != null ? `R${addr.index}.${bit}` : `R${addr.index}`;
    case "input_reg":
      return bit != null ? `IW${addr.index}.${bit}` : `IW${addr.index}`;
  }
}

/**
 * Parse strings like: I0, Q1, M2, MR5, MR1.5, R10, R1.5, %I0.0, i0
 */
export function parseVarString(raw: string): ParsedVar | null {
  const s = raw.trim().toUpperCase().replace(/^%/, "");
  const m = s.match(/^(I|Q|MR|M|R|IW|MW)(\d+)(?:\.(\d+))?$/);
  if (!m) return null;
  let token = m[1] as string;
  if (token === "MW") token = "R"; // MW = holding word (Modbus 4x)
  if (token === "IW") token = "R";
  const p = token as VarPrefix;
  if (p !== "I" && p !== "Q" && p !== "M" && p !== "MR" && p !== "R") return null;
  const index = Number(m[2]);
  if (!Number.isFinite(index) || index < 0 || index > 65535) return null;
  let bit: number | null = m[3] != null ? Number(m[3]) : null;
  if (bit != null && (bit < 0 || bit > 15)) return null;

  // Internal marker bit (M) is a pure bit area — no sub-bit.
  if (p === "M") bit = null;

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

## Prefixes

| Prefix | Meaning | Examples |
|---------|-----------|-----------|
| **I** | Discrete input | \`I0\`, \`I12\` |
| **Q** | Output / coil | \`Q0\`, \`Q3\` |
| **M** | Internal marker bit (ladder-only, **never on Modbus**) | \`M0\`, \`M12\` |
| **MR** | Internal memory register (ladder-only, **never on Modbus**) | \`MR0\`, \`MR1.3\` |
| **R** | Holding word register (Modbus 4x / MW) | \`R10\`, \`R1.3\` |

## Register bit syntax

\`\`\`
R<register_number>.<bit_number>     MR<register_number>.<bit_number>
\`\`\`

- Bit number: **0–15** (16-bit word)
- Example: \`R1.0\` = bit 0 of register R1, \`MR2.15\` = bit 15 of internal register MR2

## Simulator memory mapping

| Prefix | Memory area | On Modbus? |
|---------|----------------|-----------|
| I | Discrete Inputs (1x) | yes |
| Q | Coils (0x) | yes |
| R | Holding (4x) — word or \`.x\` bit | yes |
| **M** | Internal marker bits | **no — app only** |
| **MR** | Internal memory registers | **no — app only** |

Use **M** / **MR** for internal working memory (interlocks, sequence steps, scratch
math) that must never be readable or writable from an external Modbus master.

## Ladder elements

- **Contacts / coils** — usually **I**, **Q**, **M**, **R.n.x**
- **P edge (rising)** — one-scan pulse on 0→1
- **N edge (falling)** — one-scan pulse on 1→0
- **SET (S)** — sets a bit to 1 and holds it until RESET
- **RESET (R)** — clears a bit to 0
- **TON / CTU / Math** — **R**/**MR** registers (words), done outputs on **Q** or **M**
- **MOVE / CMP** — **R** / **MR** word operands

## Examples

| Syntax | Description |
|-------|------|
| \`I0\` | Start on input 0 |
| \`Q0\` | Output coil 0 |
| \`M5\` | Internal marker bit 5 (interlock/step) |
| \`MR20\` | Internal register 20 (setpoint, never on Modbus) |
| \`R20\` | MW20 holding word (Modbus setpoint) |
| \`R1.7\` | Bit 7 in holding register 1 |

## Tips

1. Click a ladder element to open this window.
2. Choose a prefix (I/Q/M/MR/R), number, and optional bit.
3. You can also type an address manually in the **Quick address** field (e.g. \`MR1.3\`).
4. **Apply** saves the element and updates the program (Compile & Load recommended).
`.trim();
