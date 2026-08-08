# Clean-install smoke (Windows)

Builds a release CLI binary, installs it into an isolated directory **outside** the source tree, and runs `verify-install.ps1` against that copy.

```powershell
powershell -File scripts/smoke-clean-install.ps1
```

Optional:

```powershell
powershell -File scripts/smoke-clean-install.ps1 -OutDir "$env:TEMP\rradar-clean-smoke"
```
