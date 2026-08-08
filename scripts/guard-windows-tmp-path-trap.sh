#!/usr/bin/env bash
# Guard: executable release-smoke paths must not fall back to Git-Bash /tmp
# for version JSON (v0.1.0-cli.32/.33 Windows trap).
#
# Distinguishes:
#   FAIL — executable fallback / tee under /tmp in smoke script or workflow *run* steps
#   PASS — docs, fixtures, comments, or this guard's own diagnostic strings
#
# Usage:
#   bash scripts/guard-windows-tmp-path-trap.sh           # scan repo (CI)
#   bash scripts/guard-windows-tmp-path-trap.sh --self-test
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

GUARD_SELF="scripts/guard-windows-tmp-path-trap.sh"

# Patterns that indicate a real /tmp version-JSON trap in executable paths.
# Intentionally narrow: do not treat explanatory prose as failure.
EXECUTABLE_PATTERNS=(
  'TMPDIR:-/tmp'
  '/tmp/rradar-version'
  'tee "\${TMPDIR'
  'tee /tmp/'
  'tee /tmp/rradar'
)

scan_file() {
  local file="$1"
  local hit=0
  local pat
  for pat in "${EXECUTABLE_PATTERNS[@]}"; do
    if grep -nE -- "$pat" "$file" >/dev/null 2>&1; then
      # Allow matches only when the line is purely a comment documenting the trap,
      # or when scanning the guard's own pattern table / self-test fixtures via --self-test.
      while IFS= read -r line; do
        # strip leading whitespace
        local trimmed="${line#*:}"
        trimmed="${trimmed#"${trimmed%%[![:space:]]*}"}"
        case "$trimmed" in
          \#*) continue ;;  # comment-only line: harmless
          *)
            echo "$file:$line"
            hit=1
            ;;
        esac
      done < <(grep -nE -- "$pat" "$file" || true)
    fi
  done
  return "$hit"
}

scan_repo() {
  local failed=0
  local f

  # Executable smoke script (comments mentioning /tmp are allowed).
  if [[ -f scripts/smoke-release-binary.sh ]]; then
    if scan_file scripts/smoke-release-binary.sh; then
      :
    else
      echo "error: scripts/smoke-release-binary.sh contains forbidden /tmp version-JSON fallback" >&2
      failed=1
    fi
  else
    echo "error: missing scripts/smoke-release-binary.sh" >&2
    failed=1
  fi

  # Workflow *run* bodies: scan YAML but ignore this guard step's source by
  # extracting only shell run blocks that are NOT the guard step itself.
  # Practical approach: scan workflow files excluding lines that only appear
  # inside the dedicated guard script invocation.
  for f in .github/workflows/release.yml .github/workflows/ci.yml; do
    [[ -f "$f" ]] || continue
    # Strip the guard step block (name contains "guard release smoke") before scanning,
    # and never treat docs. Also ignore comment lines.
    local tmp
    tmp="$(mktemp)"
    # Remove the guard step: from its "- name: guard release smoke..." through next "- name:" or end of jobs steps proximity.
    # Simpler and safer: only scan non-comment lines that look like shell commands (not the call to this script).
    awk '
      BEGIN { skip=0 }
      /name:[[:space:]]*guard release smoke against Windows \/tmp path trap/ { skip=1; next }
      skip==1 && /name:[[:space:]]/ { skip=0 }
      skip==1 { next }
      /^[[:space:]]*#/ { next }
      /guard-windows-tmp-path-trap\.sh/ { next }
      { print }
    ' "$f" >"$tmp"

    local hits
    hits="$(grep -nE 'TMPDIR:-/tmp|/tmp/rradar-version|tee "\$\{TMPDIR|tee /tmp/' "$tmp" || true)"
    rm -f "$tmp"
    if [[ -n "$hits" ]]; then
      echo "$f (workflow run body):"
      echo "$hits"
      echo "error: workflows must not tee/write version JSON under /tmp" >&2
      failed=1
    fi
  done

  # Required positive structure in smoke script.
  grep -q 'version --json |' scripts/smoke-release-binary.sh || {
    echo "error: smoke-release-binary.sh must pipe version --json to python stdin" >&2
    failed=1
  }
  grep -q 'SMOKE_RELEASE_BINARY_OK' scripts/smoke-release-binary.sh || {
    echo "error: smoke-release-binary.sh must emit SMOKE_RELEASE_BINARY_OK" >&2
    failed=1
  }

  return "$failed"
}

self_test() {
  local td rc=0
  td="$(mktemp -d)"

  # Negative: executable fallback must FAIL
  cat >"$td/bad-smoke.sh" <<'EOF'
#!/usr/bin/env bash
VERSION_JSON="${TMPDIR:-/tmp}/rradar-version-$$.json"
"$B" version --json | tee "$VERSION_JSON"
tee /tmp/rradar-version.json
EOF
  if scan_file "$td/bad-smoke.sh"; then
    echo "SELFTEST_FAIL: expected bad-smoke.sh to be rejected" >&2
    rc=1
  else
    echo "SELFTEST_OK: executable /tmp fallback -> FAIL"
  fi

  # Positive: platform temp / env-derived path without /tmp default -> PASS
  cat >"$td/good-smoke.sh" <<'EOF'
#!/usr/bin/env bash
# Pipe JSON; no temp file.
"$B" version --json | python -c 'import json,sys; json.load(sys.stdin)'
echo SMOKE_RELEASE_BINARY_OK
EOF
  if scan_file "$td/good-smoke.sh"; then
    echo "SELFTEST_OK: stdin pipe / no /tmp -> PASS"
  else
    echo "SELFTEST_FAIL: good-smoke.sh incorrectly rejected" >&2
    rc=1
  fi

  # Positive: docs / fixture merely mentioning /tmp -> PASS (comment-only)
  cat >"$td/doc-mention.sh" <<'EOF'
#!/usr/bin/env bash
# Historical failure: tee /tmp/rradar-version.json was unreadable on Windows.
# Do not use TMPDIR:-/tmp for version JSON.
echo ok
EOF
  if scan_file "$td/doc-mention.sh"; then
    echo "SELFTEST_OK: comment-only /tmp mention -> PASS"
  else
    echo "SELFTEST_FAIL: doc mention incorrectly rejected" >&2
    rc=1
  fi

  # Positive: guard diagnostic string in this file must not fail repo scan
  # (repo scan excludes the guard step and does not scan this script as a smoke target).
  if grep -q '/tmp/rradar-version' "$GUARD_SELF"; then
    echo "SELFTEST_OK: guard source contains diagnostic /tmp string (expected)"
  else
    echo "SELFTEST_FAIL: guard missing diagnostic pattern string" >&2
    rc=1
  fi

  rm -rf "$td"
  if [[ "$rc" -ne 0 ]]; then
    exit 1
  fi
  echo "GUARD_SELF_TEST_OK"
}

case "${1:-}" in
  --self-test)
    self_test
    ;;
  "")
    if scan_repo; then
      echo "GUARD_WINDOWS_TMP_PATH_TRAP_OK"
    else
      exit 1
    fi
    ;;
  *)
    echo "usage: $0 [--self-test]" >&2
    exit 2
    ;;
esac
