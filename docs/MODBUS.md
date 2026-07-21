# Modbus TCP

Lab-oriented Modbus TCP **slave** for SCADA / HMI integration, with an enterprise
**Translation Matrix** between PLC process image and Modbus tables.

## Defaults

| Parameter | Value |
|-----------|--------|
| Host | `127.0.0.1` |
| Port | **5020** (non-root alternative to 502) |
| Unit ID | any / default |
| Bind | `127.0.0.1:5020` by default |
| Writes | **disabled by default**; enable explicitly in the Modbus view |
| Write-protect mode | **Strict** (exception `0x02`) when a rule is write-protected |

Enable/port may be configured from the app UI (Modbus view).

## Function codes

Typical support via `tokio-modbus` server path: **FC 01–06, 15, 16** (coils, discrete, holding, write single/multi).

## Translation Matrix

```
+--------------------------+      Bidirectional      +--------------------------+
|   PLC process image      | <---------------------> |   Modbus tables          |
|  (I, Q, M, R, MR, IW)    |     Mapping Engine      | (0x, 1x, 3x, 4x)         |
+--------------------------+                         +--------------------------+
             |                                                    |
             +----------> [ Write Protect Policy Check ] <--------+
```

Rules are compiled into an O(1) index on save. Overlapping Modbus addresses are
rejected. **Internal markers (M) and memory registers (MR) are never exposed by
identity fallback** — only by an explicit enabled rule.

### Mapping types

| Type | PLC side | Modbus side | Notes |
|------|----------|-------------|-------|
| **Direct** | 1 bit or 1 word | 1 coil/DI or 1 register | Optional `length` for contiguous ranges |
| **Bit→Register** | 16 consecutive bits (e.g. M0–M15) | 1 holding/input register | LSB = `plc_start` |
| **Register→Bit** | 1 word (e.g. R10) | 16 coils/DI | Bit *i* → `modbus_start + i` |

### Write protection

Defense in depth:

1. **Global** “Allow SCADA writes” must be on (`allow_modbus_write`), or **all**
   writes return exception `ServerDeviceFailure` (`0x04`).
2. **Per-rule** `is_write_protected`:
   - **Strict** (default): write → `IllegalDataAddress` (`0x02`), PLC unchanged.
   - **Silent drop**: write is ACKed, PLC unchanged (use with care — SCADA may
     show success while the process image did not change).

Reads always succeed for mapped (or identity-fallback) addresses.

### Seed defaults (selected)

| Rule | Type | PLC | Modbus | WProt |
|------|------|-----|--------|-------|
| I0–I15 | Direct | Discrete *i* | 1x *i* | yes (RO) |
| Q0–Q15 | Direct | Coil *i* | 0x *i* | no |
| M0–M15 | Bit→Reg | MemoryBit 0 | HR 100 | no |
| R10 bits | Reg→Bit | Holding 10 | Coil 101–116 | yes |
| R500 | Direct | Holding 500 | HR 500 | yes |
| M100 | Direct | MemoryBit 100 | Coil 200 | no |
| IW0–7 | Direct | InputReg | 3x | yes |

Plus identity fallback (on by default): unmapped coil/DI/HR/IR addresses map to
the same PLC index in the matching process-image area.

## Address map sizes (process image)

| Modbus area | PLC meaning | Physical size |
|-------------|-------------|---------------|
| Coils (0x) | Q (and mapped M, etc.) | 4096 |
| Discrete inputs (1x) | I | 4096 |
| Holding registers (4x) | R (+ packed bits) | 4096 |
| Input registers (3x) | Diagnostics / IW | 1024 |

### Timer / counter holding layout (engine)

Disjoint banks so user R0… and T/C status never collide:

- Timer `timer_index = n` → `HR[2048 + 2n]` = ET (ms), `HR[2048 + 2n + 1]` = Q (0/1)
- Counter `counter_index = n` → `HR[3072 + 2n]` = CV, `HR[3072 + 2n + 1]` = Q (0/1)

Live UI compact snapshots include the first **256** bits and **128** holding/MR words
plus **32** input registers (same image from scan events and `get_memory_snapshot`).

### Diagnostic input registers

| Addr | Meaning |
|------|---------|
| IR0 | Run state (0=STOP, 1=RUN, 2=FAULT) |
| IR1 | Cycle time (ms) |
| IR2–IR3 | Scan count (low/high 16 bits) |
| IR4 | Last scan duration (µs, capped) |
| IR5 | Fault code |

## SCADA notes

- Masters can write **coils** and **holding registers** only after **Allow SCADA writes** is enabled **and** the resolved rule is not write-protected (or silent-drop applies).
- Discrete inputs and input registers are **read-only** from the master function-code set; force **I** bits from the UI process image for simulation.
- Example tools: Fuxa, Ignition, SCADA-BR on the same machine — device host `127.0.0.1`, port `5020`.

## Port conflict

```bash
lsof -i :5020   # macOS / Linux
```

Change default in `src-tauri/src/plc/modbus.rs` if needed, then rebuild.

## Security

Modbus is bound to localhost and read-only by default. Per-rule write protection
and the global write gate are lab convenience layers — do not treat this stack as
a hardened field PLC. See [SECURITY.md](../SECURITY.md).
