# OCR models

**Not bundled in this scaffold.**

After **PR-A04** (device spike) and **PR-A05**:

- Pin artifact names + **SHA-256** here
- Document Traditional Chinese pack choice and failure modes
- Provide `tools/fetch-models.sh` with hash verification
- Offline APK may embed a quantized pack under size budgets; full flavor may download with explicit UI

Never commit large weight files without a release process and license review (`docs/licenses-checklist.md` in PR-A17).
