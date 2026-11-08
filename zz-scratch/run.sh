#!/bin/sh
# usage: PREVIEW_IDS=a,b PREVIEW_PARTS=x,y PREVIEW_NAME=n [PREVIEW_SIZE=220] zz-scratch/run.sh [width] [height]
# Draws cards (base + Radiant) and kit parts into /tmp/preview/<name>.png, prints a report (CHECK lines = broken parts).
ROOT=/home/runner/work/_temp/work/item-3392
mkdir -p /tmp/preview
cd "$ROOT" && pnpm --dir apps/web exec vitest run src/cards/art/zz-preview.test.ts >/tmp/preview/vitest-${PREVIEW_NAME:-preview}.log 2>&1 || { tail -30 /tmp/preview/vitest-${PREVIEW_NAME:-preview}.log; exit 1; }
cat /tmp/preview/${PREVIEW_NAME:-preview}.txt; echo
sh "$ROOT/zz-scratch/shot.sh" "${PREVIEW_NAME:-preview}" "${1:-1600}" "${2:-1000}"
