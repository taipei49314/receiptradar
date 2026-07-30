# Security

## Reporting

Please open a private GitHub security advisory on this repository, or contact the maintainers offline. Do **not** attach real receipt photos with PII to public issues.

## Design posture

- Local-first: core CLI path does not require network
- Optional at-rest encryption: `.rrsealed` and `backup.rradar` (Argon2id + XChaCha20-Poly1305)
- No official cloud sync or hosted relay
- Default OCR is mock/local; model downloads (if any) are explicit and hash-pinned

## Threats we do not claim to stop

- Compromised device / malware with live process access
- Weak backup passphrases
- Nation-state adversaries

See `docs/design-full.md` threat model for details.
