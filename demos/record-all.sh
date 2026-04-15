#!/usr/bin/env bash
# Record all VHS demo tapes.
# Requires: vhs (https://github.com/charmbracelet/vhs)
#
# Usage:
#   ./demos/record-all.sh           # record everything
#   ./demos/record-all.sh 01 03     # record only selected tapes
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_DIR="$SCRIPT_DIR/output"
mkdir -p "$OUT_DIR"

cd "$REPO_ROOT"

# ── helpers ──────────────────────────────────────────────────────────
cleanup_port() { "$SCRIPT_DIR/helpers/fake-server.sh" stop 2>/dev/null || true; }
cleanup_docker() { docker rm -f pg 2>/dev/null || true; }

# ── collect tapes ────────────────────────────────────────────────────
if [[ $# -gt 0 ]]; then
  # Record only the tapes whose prefix matches an argument
  tapes=()
  for prefix in "$@"; do
    for f in "$SCRIPT_DIR"/${prefix}*.tape; do
      [[ -f "$f" ]] && tapes+=("$f")
    done
  done
else
  tapes=("$SCRIPT_DIR"/*.tape)
fi

if [[ ${#tapes[@]} -eq 0 ]]; then
  echo "No matching .tape files found."
  exit 1
fi

echo "Will record ${#tapes[@]} tape(s):"
printf "  %s\n" "${tapes[@]}"
echo ""

# ── record loop ──────────────────────────────────────────────────────
for tape in "${tapes[@]}"; do
  name="$(basename "$tape" .tape)"
  echo "──────────────────────────────────────────"
  echo "Recording: $name"
  echo "──────────────────────────────────────────"

  # Per-tape setup
  case "$name" in
    02-kill-port)
      "$SCRIPT_DIR/helpers/fake-server.sh" start
      sleep 1
      ;;
  esac

  # Record — output goes into demos/output/
  VHS_OUTPUT="$OUT_DIR" vhs "$tape" -o "$OUT_DIR/${name}.gif"

  # Per-tape teardown
  case "$name" in
    02-kill-port) cleanup_port ;;
    03-docker-postgres) cleanup_docker ;;
  esac

  echo "✓ $OUT_DIR/${name}.gif"
  echo ""
done

echo "All done. GIFs are in $OUT_DIR/"
