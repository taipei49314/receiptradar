# Failed tag inventory — ReceiptRadar post-`.30` release chain

Date: 2026-08-07  
Repo: https://github.com/taipei49314/receiptradar  
Policy: **Do not delete or rewrite these tags.** Owner chooses retain / withdraw / supersede.

| Tag | Target SHA | Workflow | Result | Artifact state | Recommended owner action |
|---|---|---|---|---|---|
| `v0.1.0-cli.30` | `347d2e943b8e918bcf1e3208fe032ed956839b9d` | [release #30736653007](https://github.com/taipei49314/receiptradar/actions/runs/30736653007) | **success** | Last known green Windows+unix release packaging before regression | **retain** (baseline good release) |
| `v0.1.0-cli.31` | `2d12766c72fe855093c876d2065ee5ff62c7e48b` | No tag-triggered release run listed for this ref name; master push runs [30742459216](https://github.com/taipei49314/receiptradar/actions/runs/30742459216) / [30742458457](https://github.com/taipei49314/receiptradar/actions/runs/30742458457) **failure** | **failure** (master productize) | No successful multi-OS release artifacts for this SHA under tag workflow | **retain-as-failed** (do not publish from this tag) |
| `v0.1.0-cli.32` | `9a449eb96a43dc1d274af7799bf1a3faa85e5e32` | [release #30748522909](https://github.com/taipei49314/receiptradar/actions/runs/30748522909) | **failure** — Windows `Smoke binary` | Unix builds/packages succeeded; Windows smoke failed before package/upload | **retain-as-failed**; **supersede** with next CLI tag after smoke fix |
| `v0.1.0-cli.33` | `41b76f7fde3024180ca9fe66b8c87f1b72800cc8` | [release #30755168237](https://github.com/taipei49314/receiptradar/actions/runs/30755168237) | **failure** — Windows `Smoke binary` | Same class of failure after attempted portable smoke script | **retain-as-failed**; **supersede** with next CLI tag after smoke fix |

## First reproducible failure (post-`.30`)

**Tag:** `v0.1.0-cli.32`  
**Job:** `build (rradar-x86_64-pc-windows-msvc)` → step `Smoke binary`  
**Evidence excerpt:**

```text
"$B" version --json | tee /tmp/rradar-version.json
...
{"arch":"x86_64",...,"ledger_schema":4,...,"version":"0.1.0-alpha.0"}
RELEASE_CHECK_OK schema=4 version=0.1.0-alpha.0
FileNotFoundError: [Errno 2] No such file or directory: '/tmp/rradar-version.json'
```

**Mechanism:** Git Bash can write `/tmp/...`, but native Windows `python` cannot open that path.  
**`.33` residual:** `scripts/smoke-release-binary.sh` still resolved to `/tmp/rradar-version-$$.json` when `TEMP`/`TMP` were unset in the bash step (`FileNotFoundError: '/tmp/rradar-version-1544.json'`).

## Guard added on closure branch

- `scripts/smoke-release-binary.sh` pipes `version --json` to Python **stdin** (no temp file).
- CI step `guard release smoke against Windows /tmp path trap` rejects `/tmp` version-JSON regressions.
- Soft-delete CI tees under `target/ci-trash/` instead of `/tmp`.

## Owner publish path (not executed by Cursor)

1. Merge `closure/receiptradar-20260807` after review.
2. Confirm CI green on Windows including release binary smoke.
3. Tag next candidate (suggested: `v0.1.0-cli.34`) from merged SHA — **do not reuse .31–.33**.
4. Let `release.yml` publish; verify Windows artifact uploads.
5. Leave `.31`–`.33` tags in place as failed lineage unless explicitly withdrawing GitHub Release entries (tags themselves stay).
