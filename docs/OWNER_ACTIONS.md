# OWNER_ACTIONS — receiptradar CLI release candidate

Branch: `closure/receiptradar-20260807`  
Cursor must **not** merge, force-push, delete tags, or publish the GitHub Release.

## Required owner actions

1. **Review PR** opened from `closure/receiptradar-20260807` (or create one if only the branch was pushed).
2. **Merge** only after required CI is green on Windows + Ubuntu + macOS (especially `guard release smoke…` and `release binary smoke`).
3. **Do not delete** tags `v0.1.0-cli.31`–`.33`. Prefer **supersede** with `v0.1.0-cli.34` (or next free CLI tag).
4. After merge, **create tag** `v0.1.0-cli.34` on the merged default-branch SHA and allow `release.yml` to run.
5. Confirm Windows zip + sha256 artifacts upload; smoke the downloaded binary with `scripts/verify-install.ps1 -Bin <path>`.
6. Close or supersede open PR #1 (`專案深度解析-9e691`) if its commits are already included in the closure branch.

## Explicit non-actions for Cursor

- No merge of own PR
- No force-push
- No tag delete/rewrite
- No GitHub Release publish from this agent
