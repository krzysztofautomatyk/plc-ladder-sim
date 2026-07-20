# Modbus TCP

Lab-oriented Modbus TCP **slave** for SCADA / HMI integration.

## Defaults

| Parameter | Value |
|-----------|--------|
| Host | `127.0.0.1` (or machine IP) |
| Port | **5020** (non-root alternative to 502) |
| Unit ID | any / default |
| Bind | typically `0.0.0.0:5020` |

Enable/port may be configurable from the app UI (Modbus view).

## Function codes

Typical support via `tokio-modbus` server path: **FC 01–06, 15, 16** (coils, discrete, holding, write single/multi).

## Address map (default sizes)

| Modbus area | PLC meaning | Default size |
|-------------|-------------|--------------|
| Coils (0x) | Q / markers | 4096 |
| Discrete inputs (1x) | I | 4096 |
| Holding registers (4x) | MW, timer ET, counter CV | 1024 |
| Input registers (3x) | Diagnostics | 1024 |

### Timer / counter holding layout

- Timer `timer_index = n` → `HR[2n]` = ET (ms), `HR[2n+1]` = Q (0/1)
- Counter `counter_index = n` → `HR[2n]` = CV, `HR[2n+1]` = Q (0/1)

### Diagnostic input registers

| Addr | Meaning |
|------|---------|
| IR0 | Run state (0=STOP, 1=RUN, 2=FAULT) |
| IR1 | Cycle time (ms) |
| IR2–IR3 | Scan count (low/high 16 bits) |
| IR4 | Last scan duration (µs, capped) |
| IR5 | Fault code |

## SCADA notes

- Masters write **coils** and **holding registers**.
- Discrete inputs and input registers are **read-only** from the master; force **I** bits from the UI process image for simulation.
- Example tools: Fuxa, Ignition, SCADA-BR — device host `127.0.0.1`, port `5020`.

## Port conflict

```bash
lsof -i :5020   # macOS / Linux
```

Change default in `src-tauri/src/plc/modbus.rs` if needed, then rebuild.

## Security

Exposing Modbus on a network is a lab convenience. Do not treat this stack as a hardened field PLC. See [SECURITY.md](../SECURITY.md).
