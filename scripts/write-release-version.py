#!/usr/bin/env python3
"""Write VERSION text file from rradar version --json output.

Used by .github/workflows/release.yml so schema is never hard-coded.

Usage:
  python scripts/write-release-version.py --stage DIR --tag TAG \\
      --version-json path/to/version.json
  # or pipe:
  rradar version --json | python scripts/write-release-version.py --stage DIR --tag TAG
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--stage", required=True, help="Package stage directory")
    p.add_argument("--tag", required=True, help="Git tag / release name")
    p.add_argument(
        "--version-json",
        default="",
        help="Path to version.json (default: read stdin or STAGE/version.json)",
    )
    args = p.parse_args()
    stage = Path(args.stage)
    stage.mkdir(parents=True, exist_ok=True)

    def load_text(path: Path) -> str:
        data = path.read_bytes()
        # PowerShell Out-File / Set-Content often write UTF-16 LE BOM.
        if data.startswith(b"\xff\xfe"):
            return data.decode("utf-16-le")
        if data.startswith(b"\xfe\xff"):
            return data.decode("utf-16-be")
        if data.startswith(b"\xef\xbb\xbf"):
            return data[3:].decode("utf-8")
        return data.decode("utf-8")

    if args.version_json:
        raw = load_text(Path(args.version_json))
    elif not sys.stdin.isatty():
        raw = sys.stdin.read()
    else:
        candidate = stage / "version.json"
        if candidate.is_file():
            raw = load_text(candidate)
        else:
            print("error: need --version-json, stdin, or STAGE/version.json", file=sys.stderr)
            return 2

    d = json.loads(raw.lstrip("\ufeff"))
    # Always persist machine-readable blob next to VERSION.
    (stage / "version.json").write_text(
        json.dumps(d, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    lines = [
        f"tag={args.tag}",
        f"product={d.get('product_id', 'receiptradar')}",
        f"crate_version={d.get('version', '')}",
        f"ledger_schema={d.get('ledger_schema', '')}",
        f"soft_delete={str(d.get('soft_delete', False)).lower()}",
        f"policy={d.get('policy', 'local-first; no official cloud relay')}",
        "onnx_weights=not_bundled",
        "archive_docs=LICENSE,README,INSTALL,cli,privacy,ledger-schema,RELEASE,CHANGELOG,THIRD_PARTY_NOTICES",
    ]
    text = "\n".join(lines) + "\n"
    (stage / "VERSION").write_text(text, encoding="utf-8")
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
