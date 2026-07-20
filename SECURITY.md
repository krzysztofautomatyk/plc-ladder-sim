# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 1.x     | Yes       |

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive reports.

Prefer one of:

1. GitHub **private security advisory** on this repository (if enabled)
2. Contact the maintainers via the email listed in the repository profile / About section

Include:

- Affected version / commit
- Reproduction steps
- Impact assessment (e.g. local file access, network exposure)

We aim to acknowledge reports within a reasonable time and coordinate disclosure.

## Scope & known characteristics

This application is a **desktop ladder-logic simulator** with an optional **Modbus TCP slave** for lab / training / SCADA integration demos.

- Default Modbus bind is typically **port 5020** (non-privileged). Treat it as a **lab service**, not a hardened field device.
- File-system capabilities may allow read/write under user document/home paths for import/export. Review `src-tauri/capabilities/default.json` before production-like deployment.
- Prebuilt installers from GitHub Releases are **unsigned** in v1.0 (no Apple notarization / Windows Authenticode unless maintainers add certificates later). Operating systems may show trust warnings.

## Safety disclaimer

**This software is not a certified PLC, safety controller, or medical device.**  
It is **not** certified under IEC 61508, IEC 62304, ISO 13485, or similar standards.  
Do **not** use it as the sole control path for machinery, process safety, clinical, or other life-critical systems.
