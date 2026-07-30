# backup.rradar v1

## Wire format

```text
magic:      b"RRBACKUP"          (8 bytes)
version:    u16le = 1            (2)
argon2_m:   u32le KiB            (4)  default 65536 (64 MiB)
salt:       16 bytes
nonce:      24 bytes (XChaCha20-Poly1305)
ciphertext: AEAD(plaintext) including 16-byte tag at end
```

- KDF: **Argon2id** (t=3, p=1, m from header)
- AEAD: **XChaCha20-Poly1305**, AAD = `RRBACKUP`
- Passphrase → backup key; independent of device DEK

## Plaintext archive

Length-prefixed multi-file (not tar):

```text
u32le file_count
for each file:
  u32le name_len
  name UTF-8
  u64le data_len
  data
```

Required entries:

| Name | Content |
|------|---------|
| `manifest.json` | `schema_version` (package), `created_at`, `app_version`, `transaction_count`, optional `ledger_schema_version` |
| `ledger.sqlite` | SQLite DB bytes |
| `transactions.json` | Array of transaction rows (convenience; used by `--merge` / `import backup`) |

CLI helpers: `rradar backup info|verify|restore [--merge]`. Multi-device = user-mediated file copy only (no official relay). See [ledger-schema.md](./ledger-schema.md).

## Sealed DB (P2 at-rest) `.rrsealed`

Same crypto family:

```text
magic: RRSEALED
version: u16le = 1
salt: 16
nonce: 24
ciphertext: whole SQLite file
```

AAD = `RRSEALED`. Argon2 m fixed at 64 MiB for seal/unseal of DB files.
