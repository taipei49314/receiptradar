# Ledger schema & local-first boundaries

**Policy:** ReceiptRadar is **local-first**. There is **no** official cloud sync, account, or relay. Multi-device use = encrypted `backup.rradar` / handoff packages the user copies themselves.

## Schema versioning

| Version | Binary constant | Changes |
|---------|-----------------|---------|
| 1 | historical | Base `meta` + `transactions` tables |
| 2 | | `transactions.updated_at`; meta `migrated_to_2_at`, `app_version` |
| **3** | `LEDGER_SCHEMA_VERSION = 3` | `transactions.tags`, `transactions.attachment_path` |

- On `Ledger::open`, migrations run **forward only** until the binary’s supported version.
- If the file’s version is **newer** than the binary → hard error (`SchemaTooNew`): upgrade the CLI.
- `schema_version` is stored in `meta.key = 'schema_version'`.

```bash
rradar migrate              # open db, apply steps, print version
rradar doctor               # shows supported + on-disk schema
```

## Tables (v3)

### `meta`

| key | meaning |
|-----|---------|
| `schema_version` | integer as string |
| `created_app_version` | app version when DB was first created |
| `app_version` | last migrating app version |
| `migrated_to_2_at` / `migrated_to_3_at` | ISO timestamps of migrations |

### `transactions`

Core columns from v1 (id, amounts, currency, category, invoice_id, hashes, notes, raw_text, draft_json).  
**v2+:** `updated_at` (ISO) set on insert/confirm and on edit.  
**v3+:**

| column | meaning |
|--------|---------|
| `tags` | Free-form comma-separated labels (e.g. `demo,receipt`) |
| `attachment_path` | Relative path to a receipt blob under the data dir |

Indexes: date+currency, merchant, invoice_id, content_hash.

## Attachment store (local)

Receipt images/files are **not** embedded in SQLite. They live next to the ledger:

```text
{db_parent}/attachments/{tx_id}/{safe_filename}
```

- DB stores a **relative** path: `attachments/{tx_id}/{filename}` (portable if the whole data dir moves).
- CLI: `rradar attach <id> <file>`, `rradar detach <id> [--delete-file]`, `process --confirm --attach`
- FFI: `attach_file_json` / `detach_file_json` / `resolve_attachment_path_string`
- Empty string on update **clears** tags or attachment_path.

## Backup package vs ledger schema

- **Backup package** `manifest.schema_version` = wire format of `backup.rradar` (currently `1`).
- **`manifest.ledger_schema_version`** = SQLite schema at backup time (new field; `0` if absent/legacy).
- **`manifest.attachment_count`** = number of receipt blobs packed under `attachments/**` (0 if none).

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

1. Device A: `backup create` (includes attachment blobs when present) or `handoff create`
2. User copies the encrypted file via their own channel (USB, own cloud, Syncthing, …)
3. Device B: `backup restore` / `import backup --merge` / `handoff apply` — blobs rehydrate next to the target ledger

## Explicit non-goals

- Official hosted relay / account graph  
- Automatic background sync  
- Cross-device CRDT without user-mediated backup  
