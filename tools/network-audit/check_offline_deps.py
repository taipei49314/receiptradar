#!/usr/bin/env python3
"""Offline flavor network audit stub (PR-A23).

Scans Rust sources for obvious phone-home URLs. Not a substitute for
runtime egress capture on Android offline APK.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SUSPECT = re.compile(
    r"https?://[^\s\"']+|api\.openai|firebase|sentry\.io|amplitude|mixpanel",
    re.I,
)
ALLOW = (
    "github.com/receiptradar",
    "apache.org",
    "crates.io",  # docs only in comments hopefully
    "static.rust-lang.org",
)

def main() -> int:
    hits = []
    for path in (ROOT / "crates").rglob("*.rs"):
        text = path.read_text(encoding="utf-8", errors="replace")
        for i, line in enumerate(text.splitlines(), 1):
            if line.strip().startswith("//"):
                continue
            for m in SUSPECT.finditer(line):
                url = m.group(0)
                if any(a in url for a in ALLOW):
                    continue
                # ignore pure documentation strings in help text about github
                if "github.com" in url and "receiptradar" not in url:
                    hits.append(f"{path.relative_to(ROOT)}:{i}: {url}")
                elif "github.com" not in url:
                    hits.append(f"{path.relative_to(ROOT)}:{i}: {url}")
    if hits:
        print("SUSPECT network references in non-comment code:")
        for h in hits:
            print(" ", h)
        return 1
    print("network-audit: no suspect non-comment URLs in crates/**/*.rs")
    return 0

if __name__ == "__main__":
    sys.exit(main())
