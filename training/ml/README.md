# training/ml

The learned evaluation's fit (#466, #1009; `docs/learned-eval.md` D4, D5, D8): Python and CPU-only
PyTorch over the positions `cargo jackioh positions` writes. It is kept apart from `analysis/`, so
`super.yml`'s notebooks job never installs torch. Nothing in `crates/`, `apps/`, `e2e/` or `spec/`
reads it; what it makes reaches the AI only as `crates/ai/src/learned/weights.json`, through a
reviewed pull request (`export.py`, #1010).

```
uv sync --frozen --project training/ml                             # Python 3.13, torch (CPU) and the pins, into training/ml/.venv
uv run --project training/ml pytest training/ml/tests              # the loader and one fit end to end, on the committed fixture
uv run --project training/ml python -m jackioh_ml.fit <positions>… --out training/ml/out/<name>
```

`uv` is `python3 -m pip install --user uv` away. Every dependency is pinned in `pyproject.toml` and
`uv.lock`, torch from PyTorch's CPU index (`[[tool.uv.index]]`); `--frozen` installs exactly the lock.
To move a pin, edit `pyproject.toml`, run `uv lock --project training/ml` and commit both files.

## Where the data comes from

1. An export: `gh workflow run selfplay.yml -f games=100000 -f seed=<base>` on GitHub's runners or the
   local runner (`training/README.md`, Where the exports come from), or by hand:
   `cargo jackioh selfplay --games 2000 --seed <base> --out <dir>` (R1006001, R1006002).
2. Positions: `cargo jackioh positions <dir>/selfplay-*.jsonl --out positions.jsonl` (one row per seat
   per turn, the default; `--every decision` for every decision), which drops and counts any game that
   no longer replays (R1006003, R1006004).

`training/ml/data/` and `training/ml/out/` are git-ignored: positions and fits are data. The fixture
`tests/fixtures/positions.jsonl` (20 games, `selfplay --games 20 --seed ml-fixture`, 284 positions) is
the only data committed.

## The fit

`python -m jackioh_ml.fit` (`jackioh_ml/fit.py`) loads every file given (`data.load`: the feature
length from the data; a `FEATURES_VERSION` or dense layout that differs between files is refused),
holds 10% of the games out whole by a hash of their seed (`data.split_by_game`; a game's positions
never sit on both sides), keeps another 10% for early stopping, and fits:

| | Model | Fit |
|---|---|---|
| (a) | the hand eval calibrated to a probability: `σ(a·hand/100 + b)` | L-BFGS to convergence |
| (b) | a logistic over the dense features and the hand eval, session s15's linear fit | L-BFGS, its L2 picked on the validation games |
| (c) | D5's network with D4's skip connection: `σ(a·hand/100 + b + net(x))`, its first layer 16 wide over the standardized dense features and one row per sparse key seen in 20 or more positions, a clipped ReLU, a head of 16, starting from (a) with its output at zero | Adam on the log loss, early stopping on the validation games, three seeds |

`<out>/report.md` (and `report.json`) holds: held-out log loss, AUC and calibration, overall and by
turn band (1-3, 4-10, 11 and on); (c)'s run-to-run spread over its seeds; (c)'s learning curve at
10,000, 30,000 and 100,000 games (as many as the data holds); the hand eval's most misjudged cards on
the seat's board, both ways; and #1009's decision:

- **go** when (c) beats (a) and (b) on held-out log loss in turns 4-10 by more than its spread;
- **more data** when the curve is still falling at its largest size;
- **stop** otherwise: more compute will not help, and the features or the model must change.

`<out>/model.pt` keeps the best seed's network with its vocabulary and standardizer, for `export.py`.

## The integer network

`python -m jackioh_ml.quantize --weights <float weights.json> --out <integer weights.json>
[--positions <file> --report <report.json>]` (#1011) makes the network that ships: the standardizer
folded into an i16 first layer over the raw dense features, i8 head layers, i64 sums, floor shifts and
clips to `[0, 127]`, and one division into the logit (R1011000). `--report` gives the integer network's
held-out log loss beside the float one's and their largest difference in logits.
`--random 1011 --positions tests/fixtures/positions.jsonl --parity
crates/ai/tests/fixtures/learned-parity.json` writes the parity fixture both builds are held to.
