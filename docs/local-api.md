# Local HTTP API (`rradar serve`)

**Loopback only.** No cloud relay, no public bind, no account.

```bash
rradar serve --bind 127.0.0.1:7432 --db path/to/ledger.db

# One-command product smoke (ephemeral port, isolated/temp db by default):
rradar api-smoke --fixtures fixtures
```

Non-loopback binds are **rejected** (CLI + server).

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` or `/` | `ok` |
| GET | `/version` | product/version/schema JSON (`local_only: true`) |
| GET | `/capabilities` | feature flags (no cloud, attachment store, …) |
| GET | `/paths` | db / attachments / inbox paths |
| GET | `/transactions?limit=&currency=&q=` | recent transactions JSON (filter optional) |
| GET | `/transaction?id=` | one transaction JSON |
| GET | `/stats` | per-currency totals JSON |
| GET | `/report?y=2024&m=5` | monthly markdown report |
| GET | `/models` | ONNX pin status JSON (no weights uploaded) |
| POST | `/process` | parse path; optional confirm + attach + tags |
| POST | `/attach` | copy file into attachment store for an id |
| OPTIONS | `*` | CORS preflight for `null` origin (local file demos) |

### POST `/process` body

```json
{
  "path": "C:/receipts/photo.jpg",
  "confirm": true,
  "attach": true,
  "tags": "demo,inbox",
  "engine": "mock",
  "currency": "TWD",
  "force": false
}
```

- `confirm: false` → returns draft JSON only  
- `confirm: true` → inserts into ledger; with `attach: true` copies source under `{db_parent}/attachments/{id}/`

### POST `/attach` body

```json
{ "id": "01H…", "path": "C:/receipts/photo.jpg" }
```

## Product paths (local multi-device)

| Path | Tool |
|------|------|
| Drop folder | `rradar inbox --ensure` + `rradar watch [--attach]` |
| Encrypted file | `rradar backup create` / `import backup` |
| Local API | `rradar serve` / `rradar api-smoke` (this document) |
| Demo closed loop | `rradar demo` (includes API smoke step) |

Still **not** provided: official sync, hosted relay, accounts.

## Offline network audit

`tools/network-audit/check_offline_deps.py` fails the build on remote phone-home URLs.  
Loopback placeholders and `127.0.0.1` / `localhost` are allowed for this API. Banner lines intentionally avoid the substring `http://`.
