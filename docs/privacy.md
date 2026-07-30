# Privacy — ReceiptRadar

**Status:** outline for v0.1 (Track A). Expand before public binary (PR-A17/A22/A25).

## Principles

1. **Local-first by default** — receipt images, OCR text, and ledger rows stay on device.
2. **No account** for core features.
3. **No silent network** — models are never downloaded without an explicit user action (or are fully embedded in the offline flavor).
4. **Opt-in only** for any feature that leaves the device (export is user-driven; future SMS parsers etc. remain optional).
5. **No official sync relay** — multi-device = encrypted backup / export only (project policy).

## Network modes

| Mode | Build | Network | Notes |
|------|-------|---------|-------|
| **A** | `receiptradar-offline` | No `INTERNET` permission | Airplane-mode capable |
| **B** | `full` | User-initiated model download | Hash-pinned assets only |
| **C** | `full` + toggles | Explicit feature flags | Defaults **off** |

CI will include an **egress audit** for offline configurations (PR-A23).

## Data inventory (device)

| Data | Location | Leaves device? |
|------|----------|----------------|
| Receipt images | Encrypted blobs (v0.1) | Only if user exports backup / shares file |
| OCR text / drafts | SQLite (encrypted at rest in v0.1) | Same |
| Categories / overrides | SQLite | Same |
| Models | App storage / assets | Downloaded only with consent (mode B) |

## Retention

- User setting: wipe images after N days or after successful extract (to be implemented in mobile settings, PR-A21).
- Uninstall / reinstall: at-rest keys in Android Keystore are **not** recoverable — data loss is expected and documented in UX.

## Debug & support

- Prefer `rradar process --explain` (PR-A12) over sharing raw photos.
- Crash reporting, if ever added, is **opt-in**, redacted, and off by default.
- Do not attach live receipts to public issues without redaction.

## Store / F-Droid

- v0.1 distribution: **sideload + GitHub Releases**; F-Droid **offline** flavor is the privacy-oriented target.
- Google Play is **not** a v0.2 requirement.

## FLAG_SECURE

- Default **ON** (blocks screenshots / recents preview of amounts).
- User may disable in Settings (KD-23).

## Threat model (summary)

| Threat | Mitigation |
|--------|------------|
| Cloud SaaS exfil | No cloud core path |
| Device thief | At-rest encryption + session auto-lock (5 min background default) |
| Supply-chain models | SHA-256 pin + notices |
| Curious backup holder | Passphrase Argon2id on `backup.rradar` |
| Official relay abuse | **N/A** — we do not run one |

We do **not** claim resistance to nation-state malware on a compromised device.
