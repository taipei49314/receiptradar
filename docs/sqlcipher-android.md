# SQLCipher / at-rest (A16)

## Decision for current desktop Track A

| Priority | Path | Status |
|----------|------|--------|
| P1 | rusqlite + SQLCipher NDK amalgamation | Deferred to mobile FFI (Android NDK) |
| **P2 (implemented now)** | Plain SQLite pages + **whole-file AEAD** `.rrsealed` | **Active** for CLI |

P2 matches design Yellow path when P1 linkage is not available (no MSVC/NDK in this environment). Threat model: device thief without passphrase cannot read ledger at rest; temp decrypt files should live in OS private temp and be deleted after use.

## CLI

```bash
rradar seal --db ledger.db --out ledger.rrsealed --passphrase '…'
rradar list --db ledger.rrsealed --passphrase '…'
rradar process photo.txt --confirm --db ledger.rrsealed --passphrase '…'
```

## Mobile (future)

See design KD-17 / key ladder: Keystore-wrapped DEK + SQLCipher or sealed file; session lock 5 minutes.
