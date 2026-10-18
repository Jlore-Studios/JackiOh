#!/usr/bin/env bash
# The policy prior's retrain, start to end, with no LLM inside it (#1171; crates/ai/src/policy.rs,
# R1171000, R1171001), on retrain.sh's pattern: one script exports, labels, fits, screens, and names the
# gate only after a passing screen.
#
#   training/ml/retrain_policy.sh --out <dir> (--logs <dir> | --games <n> --seed <base>) [--parent-bin <path>]
#                                 [--screen-games 800]
#
#   1. the logs: `--logs` reuses an export (selfplay.yml's artifacts, or a local run), else `jackioh
#      selfplay --games <n> --seed <base>` plays one here (R1006001, R1006002);
#   2. `jackioh positions --every decision --policy` over them: every main-phase decision labelled with
#      the AI's candidates there and the move played (R1171000; games that no longer replay are dropped
#      and counted, R1006003);
#   3. the fit (`python -m jackioh_ml.policy_fit`) and its report; it stops here unless the prior puts
#      the held-out played move in the first six more often than the heuristic order (`decision.gain` > 0);
#   4. the table into crates/ai/src/policy/weights.json, and the switch, AI_MOVE_ORDER, set to Learned in
#      crates/ai/src/config.rs;
#   5. the parent: `--parent-bin <path>`; `--parent-bin tree`, the exporter of step 1, built from this
#      tree before the table changed (when the tree is main); else `jackioh` built from
#      origin/main in a worktree of its own (never this tree's target/, #585);
#   6. the candidate: this tree's `jackioh`, built with the new table;
#   7. the screen: `arena --a self --b bin:<parent> --set improve-dev --games <screen-games>` (800 by
#      default), a count decided before it is played and never extended or played again to get a
#      better read (training/improve.md, How you measure).
#
# It writes everything under <out> (`summary.json` last) and leaves the table and the switch changed in
# the working tree. When the screen's `edgeLow` is above zero it prints the gate's command and exits 0;
# otherwise it exits 1 and the gate is not run: the gate's hold-out deals are read once per tree
# (R850001), by a candidate a screen has already passed.
set -euo pipefail

out="" logs="" games="" seed="" parent="" screen_games=800
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --logs) logs="$2"; shift 2 ;;
    --games) games="$2"; shift 2 ;;
    --seed) seed="$2"; shift 2 ;;
    --parent-bin) parent="$2"; shift 2 ;;
    --screen-games) screen_games="$2"; shift 2 ;;
    *) echo "retrain_policy.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$out" ] || { echo "retrain_policy.sh: --out is required" >&2; exit 2; }
if [ -z "$logs" ] && { [ -z "$games" ] || [ -z "$seed" ]; }; then
  echo "retrain_policy.sh: name --logs, or --games and --seed" >&2; exit 2
fi

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p "$out"
out=$(cd "$out" && pwd)
say() { echo "retrain_policy: $*"; }

say "building jackioh from this tree (the exporter)"
cargo build --release -p jackioh-tools --quiet
bin="${CARGO_TARGET_DIR:-$root/target}/release/jackioh"
cp "$bin" "$out/tree-jackioh"

if [ -z "$logs" ]; then
  logs="$out/logs"
  say "playing $games games on $seed"
  "$bin" selfplay --games "$games" --seed "$seed" --out "$logs" | tee "$out/selfplay.json"
fi
mapfile -t files < <(find "$logs" -name 'selfplay-*.jsonl' | sort -V)
[ "${#files[@]}" -gt 0 ] || { echo "retrain_policy.sh: no selfplay-*.jsonl under $logs" >&2; exit 1; }

say "positions (--every decision --policy) over ${#files[@]} log files"
"$bin" positions "${files[@]}" --out "$out/positions.jsonl" --every decision --policy | tee "$out/positions.json"

# The trainer's package lives in training/ml, which `python -m` reads only from the path.
export PYTHONPATH="$root/training/ml${PYTHONPATH:+:$PYTHONPATH}"

say "the fit"
uv run --frozen --project training/ml python -m jackioh_ml.policy_fit "$out/positions.jsonl" --out "$out/fit"
gain=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['decision']['gain'])" "$out/fit/report.json")
if ! python3 -c "import sys; sys.exit(0 if float(sys.argv[1]) > 0 else 1)" "$gain"; then
  say "the prior does not put the held-out move played in the first six more often (gain $gain): no screen"
  printf '{"stage":"fit","gain":%s,"screened":false}\n' "$gain" > "$out/summary.json"
  exit 1
fi

say "the table and the switch"
cp "$out/fit/weights.json" crates/ai/src/policy/weights.json
sed -i 's/^pub const AI_MOVE_ORDER: MoveOrder = MoveOrder::Heuristic;/pub const AI_MOVE_ORDER: MoveOrder = MoveOrder::Learned;/' \
  crates/ai/src/config.rs
grep -q '^pub const AI_MOVE_ORDER: MoveOrder = MoveOrder::Learned;' crates/ai/src/config.rs

if [ "$parent" = tree ]; then
  cp "$out/tree-jackioh" "$out/parent-jackioh"
  parent="$out/parent-jackioh"
elif [ -z "$parent" ]; then
  say "the parent: jackioh from origin/main"
  git fetch --quiet origin main
  rm -rf "$out/parent-src"
  git worktree add --quiet --detach "$out/parent-src" origin/main
  cargo build --release -p jackioh-tools --quiet --manifest-path "$out/parent-src/Cargo.toml" \
    --target-dir "$out/parent-target"
  cp "$out/parent-target/release/jackioh" "$out/parent-jackioh"
  git worktree remove --force "$out/parent-src"
  parent="$out/parent-jackioh"
fi

say "the candidate: this tree with the new table"
cargo build --release -p jackioh-tools --quiet
cp "$bin" "$out/candidate-jackioh"

say "the screen: $screen_games paired games on improve-dev, decided in advance"
"$out/candidate-jackioh" arena --a self --b "bin:$parent" --set improve-dev --games "$screen_games" \
  --out "$out/screen-records" > "$out/screen.txt"
python3 - "$out" "$gain" <<'PY'
import json, sys
out, gain = sys.argv[1], float(sys.argv[2])
tally, worst = None, 0
for line in open(f"{out}/screen.txt", encoding="utf-8"):
    line = line.strip()
    if not line.startswith("{"):
        continue
    row = json.loads(line)
    if "edgeLow" in row:
        tally = row
    elif "msP95" in row:
        worst = max(worst, row["msP95"])
summary = {"stage": "screen", "gain": gain, "screened": True, "tally": tally, "maxMsP95": worst,
           "passed": bool(tally and tally["edgeLow"] > 0)}
json.dump(summary, open(f"{out}/summary.json", "w"), indent=1)
print(json.dumps(summary))
PY
if python3 -c "import json,sys; sys.exit(0 if json.load(open(sys.argv[1]))['passed'] else 1)" "$out/summary.json"; then
  say "the screen passed (edgeLow > 0). Commit the table and the switch, then play the gate once:"
  echo "  cargo jackioh promote --lane learn --parent-bin $parent"
  exit 0
fi
say "the screen did not pass: no gate"
exit 1
