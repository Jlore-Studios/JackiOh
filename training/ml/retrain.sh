#!/usr/bin/env bash
# The learned evaluation's retrain, start to end, with no LLM inside it (#466, #1012;
# docs/learned-eval.md D2): one script exports, fits, screens, and names the gate only after a
# passing screen. A person, the learn lane's workflow (.github/workflows/learn.yml) or a session runs
# it; whoever proposes a change to the features or the model measures it with this same script.
#
#   training/ml/retrain.sh --out <dir> (--logs <dir> | --games <n> --seed <base>) [--parent-bin <path>]
#                          [--screen-games 800] [--every turn|decision] [--seeds "1 2 3"]
#
#   1. the logs: `--logs` reuses an export (selfplay.yml's artifacts, or a local run), else `jackioh
#      selfplay --games <n> --seed <base>` plays one here (R1006001, R1006002);
#   2. `jackioh positions` over them (games that no longer replay are dropped and counted, R1006003);
#   3. the fit (`python -m jackioh_ml.fit`), and its report; it stops here unless the network beats the
#      calibrated hand eval on held-out turns 4-10 (`decision.gain` > 0);
#   4. the weights (`python -m jackioh_ml.export`) into crates/ai/src/learned/weights.json, and the
#      switch, AI_EVAL_KIND, set to Learned in crates/ai/src/config.rs;
#   5. the parent: `--parent-bin <path>`; `--parent-bin tree`, the exporter of step 1, built from this
#      tree before the weights changed (when the tree is main, as in learn.yml); else `jackioh` built from
#      origin/main in a worktree of its own (never this tree's target/, #585);
#   6. the candidate: this tree's `jackioh`, built with the new weights;
#   7. the screen: `arena --a self --b bin:<parent> --set improve-dev --games <screen-games>` (800 by
#      default), a count decided before it is played and never extended or played again to get a
#      better read (training/improve.md, How you measure).
#
# It writes everything under <out> (`summary.json` last) and leaves the weights and the switch changed
# in the working tree. When the screen's `edgeLow` is above zero it prints the gate's command and exits
# 0; otherwise it exits 1 and the gate is not run: the gate's hold-out deals are read once per tree
# (R850001), by a candidate a screen has already passed.
set -euo pipefail

out="" logs="" games="" seed="" parent="" screen_games=800 every=turn seeds="1 2 3"
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --logs) logs="$2"; shift 2 ;;
    --games) games="$2"; shift 2 ;;
    --seed) seed="$2"; shift 2 ;;
    --parent-bin) parent="$2"; shift 2 ;;
    --screen-games) screen_games="$2"; shift 2 ;;
    --every) every="$2"; shift 2 ;;
    --seeds) seeds="$2"; shift 2 ;;
    *) echo "retrain.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$out" ] || { echo "retrain.sh: --out is required" >&2; exit 2; }
if [ -z "$logs" ] && { [ -z "$games" ] || [ -z "$seed" ]; }; then
  echo "retrain.sh: name --logs, or --games and --seed" >&2; exit 2
fi

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
mkdir -p "$out"
out=$(cd "$out" && pwd)
say() { echo "retrain: $*"; }

say "building jackioh from this tree (the exporter)"
cargo build --release -p jackioh-tools --quiet
bin="$root/target/release/jackioh"
cp "$bin" "$out/tree-jackioh"

if [ -z "$logs" ]; then
  logs="$out/logs"
  say "playing $games games on $seed"
  "$bin" selfplay --games "$games" --seed "$seed" --out "$logs" | tee "$out/selfplay.json"
fi
mapfile -t files < <(find "$logs" -name 'selfplay-*.jsonl' | sort -V)
[ "${#files[@]}" -gt 0 ] || { echo "retrain.sh: no selfplay-*.jsonl under $logs" >&2; exit 1; }

say "positions (--every $every) over ${#files[@]} log files"
"$bin" positions "${files[@]}" --out "$out/positions.jsonl" --every "$every" | tee "$out/positions.json"

say "the fit"
# shellcheck disable=SC2086  # the seeds are words
uv run --frozen --project training/ml python -m jackioh_ml.fit "$out/positions.jsonl" --out "$out/fit" \
  --root "$root" --seeds $seeds
gain=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['decision']['gain'])" "$out/fit/report.json")
if ! python3 -c "import sys; sys.exit(0 if float(sys.argv[1]) > 0 else 1)" "$gain"; then
  say "the network does not beat the calibrated hand eval on held-out turns 4-10 (gain $gain): no screen"
  printf '{"stage":"fit","gain":%s,"screened":false}\n' "$gain" > "$out/summary.json"
  exit 1
fi

say "the weights and the switch"
uv run --frozen --project training/ml python -m jackioh_ml.export --model "$out/fit/model.pt" \
  --out crates/ai/src/learned/weights.json
sed -i 's/^pub const AI_EVAL_KIND: EvalKind = EvalKind::Hand;/pub const AI_EVAL_KIND: EvalKind = EvalKind::Learned;/' \
  crates/ai/src/config.rs
grep -q '^pub const AI_EVAL_KIND: EvalKind = EvalKind::Learned;' crates/ai/src/config.rs

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

say "the candidate: this tree with the new weights"
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
  say "the screen passed (edgeLow > 0). Commit the weights and the switch, then play the gate once:"
  echo "  cargo jackioh promote --lane learn --parent-bin $parent"
  exit 0
fi
say "the screen did not pass: no gate"
exit 1
