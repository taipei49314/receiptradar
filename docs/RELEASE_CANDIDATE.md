# Release candidate — ReceiptRadar CLI (not published)

**Candidate target tag (owner-created only):** `v0.1.0-cli.34`  
**Branch:** `closure/receiptradar-20260807`  
**Product claim:** CLI bookkeeping product release candidate. Flutter/mobile explicitly out of completed scope.

## Included

- PR #1 attachment purge lifecycle (database-first cleanup + PurgeReport)
- Lifecycle regressions: soft-delete→purge, missing file, orphan blob, interrupted DB delete, backup→restore→read (CLI)
- Windows release smoke fix: `version --json` piped to Python stdin (root cause of `.32`/`.33` failures)
- CI guard against `/tmp` version-JSON regressions
- `docs/FAILED_TAG_INVENTORY.md`, `docs/OWNER_ACTIONS.md`
- `scripts/smoke-clean-install.ps1`

## Not included / not claimed

- Flutter mobile ship
- Publishing GitHub Release or deleting failed tags
- ONNX weights bundled in archive

## Local verification (this machine)

See external evidence bundle `closure-output/receiptradar/<sha>/` for exact commands, exit codes, and checksums after build.
