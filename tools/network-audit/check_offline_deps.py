#!/usr/bin/env python3
"""Offline flavor network audit (PR-A23).

Scans Rust sources for phone-home URLs. Allows **loopback-only** local HTTP
patterns used by `rradar serve` (127.0.0.1 / localhost / format placeholders).

Not a substitute for runtime egress capture on Android offline APK.
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

# Substrings that make a hit acceptable (docs hosts + local-only serve).
ALLOW_SUBSTR = (
    "github.com/taipei49314",
    "github.com/receiptradar",
    "apache.org",
    "crates.io",
    "static.rust-lang.org",
    "127.0.0.1",
    "localhost",
    "[::1]",
    "0.0.0.0",  # only if paired with refuse-non-loopback elsewhere; still flag if remote host
)

# Entire match is a format placeholder for bind display, not a remote URL.
ALLOW_EXACT = re.compile(
    r"^https?://\{\}(/.*)?$"  # http://{}  or http://{}/path
    r"|^https?://\{bind\}"  # named format
    r"|^https?://%s",
    re.I,
)


def is_allowed(url: str) -> bool:
    u = url.strip().rstrip("\",')")
    if ALLOW_EXACT.match(u):
        return True
    if any(a in u for a in ALLOW_SUBSTR):
        # Still block obvious third-party analytics even if substring noise
        if re.search(r"openai|firebase|sentry\.io|amplitude|mixpanel", u, re.I):
            return False
        # Allow github only for our org / generic docs mentions handled below
        if "github.com" in u and "taipei49314" not in u and "receiptradar" not in u:
            return False
        return True
    # Pure github.com without our org → report (same as before)
    if "github.com" in u:
        return False
    return False


def main() -> int:
    hits = []
    for path in (ROOT / "crates").rglob("*.rs"):
        text = path.read_text(encoding="utf-8", errors="replace")
        for i, line in enumerate(text.splitlines(), 1):
            if line.strip().startswith("//"):
                continue
            for m in SUSPECT.finditer(line):
                url = m.group(0)
                if is_allowed(url):
                    continue
                hits.append(f"{path.relative_to(ROOT)}:{i}: {url}")
    if hits:
        print("SUSPECT network references in non-comment code:")
        for h in hits:
            print(" ", h)
        print(
            "hint: local serve must use 127.0.0.1/localhost or http://{} bind placeholders only"
        )
        return 1
    print("network-audit: no suspect non-comment URLs in crates/**/*.rs")
    return 0


if __name__ == "__main__":
    sys.exit(main())
