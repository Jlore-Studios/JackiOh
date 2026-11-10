#!/bin/sh
# One command for the busy-board frame probe (issue #3605): builds the e2e client, serves it
# with `vite preview` on :4173 and runs spec 50 against it, printing both phases' numbers.
# `--json` prints the same report as one JSON object per phase (`probeJson`, as spec 25's
# `shots`). The spec, not this script, judges the budgets: a phase over its p95 or long-task
# budget fails the run, naming the constant.
#
#   pnpm --dir e2e probe:frames
#   pnpm --dir e2e probe:frames -- --json
set -eu

JSON=0
for arg in "$@"; do
  case "$arg" in
    # pnpm passes its own `--` separator through (`probe:frames -- --json`).
    --) ;;
    --json) JSON=1 ;;
    *)
      echo "usage: sh scripts/frame-probe.sh [--json]" >&2
      exit 2
      ;;
  esac
done

PORT=4173

pnpm --dir ../apps/web build:e2e

pnpm --dir ../apps/web exec vite preview --port "$PORT" --strictPort >"${TMPDIR:-/tmp}/jackioh-frame-probe-preview.log" 2>&1 &
PREVIEW_PID=$!
stop_preview() {
  kill "$PREVIEW_PID" 2>/dev/null || true
  # The pid above is pnpm's wrapper: take the port's listener too, when lsof exists.
  if command -v lsof >/dev/null 2>&1; then
    stray="$(lsof -tiTCP:"$PORT" -sTCP:LISTEN || true)"
    if [ -n "$stray" ]; then kill $stray 2>/dev/null || true; fi
  fi
}
trap stop_preview EXIT INT TERM

i=0
while [ "$i" -lt 60 ]; do
  if curl -sf "http://localhost:$PORT/" >/dev/null; then break; fi
  i=$((i + 1))
  sleep 1
done
curl -sf "http://localhost:$PORT/" >/dev/null

if [ "$JSON" -eq 1 ]; then
  # shellcheck disable=SC2086
  E2E_BASE_URL="http://localhost:$PORT" pnpm exec cypress run --browser chrome --spec cypress/e2e/50-busy-board-frames.cy.ts --expose probeJson=1
else
  E2E_BASE_URL="http://localhost:$PORT" pnpm exec cypress run --browser chrome --spec cypress/e2e/50-busy-board-frames.cy.ts
fi
