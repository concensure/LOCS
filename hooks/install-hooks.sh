#!/usr/bin/env bash
# Wires the LOCS pre-commit hook into the current git repo.
# Run once from the root of any project that uses locs.py.
#
# Usage:
#   bash hooks/install-hooks.sh
#   bash hooks/install-hooks.sh --uninstall

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "")"

if [[ -z "$REPO_ROOT" ]]; then
  echo "ERROR: not inside a git repository"
  exit 1
fi

HOOK_SRC="$(cd "$(dirname "$0")" && pwd)/pre-commit"
HOOK_DST="$REPO_ROOT/.git/hooks/pre-commit"

if [[ "${1:-}" == "--uninstall" ]]; then
  if [[ -f "$HOOK_DST" ]] && grep -q "locs" "$HOOK_DST"; then
    rm "$HOOK_DST"
    echo "uninstalled LOCS pre-commit hook"
  else
    echo "no LOCS hook found at $HOOK_DST"
  fi
  exit 0
fi

if [[ ! -f "$HOOK_SRC" ]]; then
  echo "ERROR: hook source not found at $HOOK_SRC"
  exit 1
fi

# if a pre-commit hook already exists and is not ours, append rather than overwrite
if [[ -f "$HOOK_DST" ]] && ! grep -q "locs" "$HOOK_DST"; then
  echo "" >> "$HOOK_DST"
  cat "$HOOK_SRC" >> "$HOOK_DST"
  echo "appended LOCS validation to existing pre-commit hook at $HOOK_DST"
else
  cp "$HOOK_SRC" "$HOOK_DST"
  chmod +x "$HOOK_DST"
  echo "installed LOCS pre-commit hook at $HOOK_DST"
fi

echo "test with: git commit --dry-run"
