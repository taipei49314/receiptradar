# Local HTTP API (`rradar serve`)

**Loopback only.** No cloud relay, no public bind, no account.

```bash
rradar serve --bind 127.0.0.1:7432 --db path/to/ledger.db
```

Non-loopback binds are **rejected** (CLI + server).

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` or `/` | `ok` |
| GET | `/version` | product/version JSON (`local_only: true`) |
| GET | `/transactions` | recent transactions JSON |
| GET | `/stats` | per-currency totals JSON |
| GET | `/report?y=2024&m=5` | monthly markdown report |
| GET | `/models` | ONNX pin status JSON (no weights uploaded) |
| POST | `/process` | body `{"path":"...","confirm":false,"engine":"mock"}` |

## Product paths (local multi-device)

| Path | Tool |
|------|------|
| Drop folder | `rradar inbox --ensure` + `rradar watch` |
| Encrypted file | `rradar backup create` / `import backup` |
| Local API | `rradar serve` (this document) |

Still **not** provided: official sync, hosted relay, accounts.

## Offline network audit

`tools/network-audit/check_offline_deps.py` fails the build on remote phone-home URLs.  
Loopback placeholders and `127.0.0.1` / `localhost` are allowed for this API.
