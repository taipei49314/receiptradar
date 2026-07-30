# Ledger schema & local-first boundaries

**Policy:** ReceiptRadar is **local-first**. There is **no** official cloud sync, account, or relay. Multi-device use = encrypted `backup.rradar` / export files the user copies themselves.

## Schema versioning

| Version | Binary constant | Changes |
|---------|-----------------|---------|
| 1 | historical | Base `meta` + `transactions` tables |
| **2** | `LEDGER_SCHEMA_VERSION = 2` | `transactions.updated_at`; meta `migrated_to_2_at`, `app_version` |

- On `Ledger::open`, migrations run **forward only** until the binary’s supported version.
- If the file’s version is **newer** than the binary → hard error (`SchemaTooNew`): upgrade the CLI.
- `schema_version` is stored in `meta.key = 'schema_version'`.

```bash
rradar migrate              # open db, apply steps, print version
rradar doctor               # shows supported + on-disk schema
```

## Tables (v2)

### `meta`

| key | meaning |
|-----|---------|
| `schema_version` | integer as string |
| `created_app_version` | app version when DB was first created |
| `app_version` | last migrating app version |
| `migrated_to_2_at` | ISO timestamp of v2 migration |

### `transactions`

Core columns unchanged from v1 (id, amounts, currency, category, invoice_id, hashes, notes, raw_text, draft_json).  
**v2+:** `updated_at` (ISO) set on insert/confirm and on edit.

Indexes: date+currency, merchant, invoice_id, content_hash.

## Backup package vs ledger schema

- **Backup package** `manifest.schema_version` = wire format of `backup.rradar` (currently `1`).
- **`manifest.ledger_schema_version`** = SQLite schema at backup time (new field; `0` if absent/legacy).

```bash
rradar backup create -p 'secret' -o month.rradar
rradar backup info --in month.rradar -p 'secret'
rradar backup verify --in month.rradar -p 'secret'
rradar backup restore --in month.rradar -p 'secret' --db restored.db
rradar backup restore --in month.rradar -p 'secret' --merge   # merge into existing
rradar import backup --in month.rradar -p 'secret'            # same as --merge
```

See also: [backup-format-v1.md](./backup-format-v1.md).

## Multi-device (allowed)

1. Device A: `backup create`
2. User copies `.rradar` via their own channel (USB, own cloud, Syncthing, …)
3. Device B: `backup restore` or `import backup --merge`

## Explicit non-goals

- Official hosted relay / account graph  
- Automatic background sync  
- Cross-device CRDT without user-mediated backup  
