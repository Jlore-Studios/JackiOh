# analysis

Python over JackiOh's game records (SPEC §9.11, R376–R378): a loader that turns record files into pandas
tables, R377's card win rates computed over them, and the project the notebooks run in. Nothing in `crates/`,
`apps/`, `e2e/` or `spec/` reads this directory, and nothing here changes a rule, the wire, the database or
what a player sees: it reads records and prints figures.

```
uv sync --frozen --project analysis                    # Python 3.13 and the pinned dependencies, into analysis/.venv
uv run --project analysis pytest analysis/tests        # the loader's tests (the card figures equal the Rust report's) and every notebook, run
uv run --project analysis jupyter lab                  # the notebooks' server, from the repository root
```

`uv` is `pip install uv` away (`python3 -m pip install --user uv`). Every dependency is pinned to one version
in `pyproject.toml`, and `uv.lock` pins the rest; `--frozen` installs exactly the lock and never edits it. To
move a pin, edit `pyproject.toml`, run `uv lock --project analysis` and commit both files.

## Where the records come from

A record file is one `GameRecord` (R376) per line, `source` `live` or `dev`, whoever wrote it:

| Records | Written by |
|---|---|
| Live games, off `public.game_records` | `jackioh-server stats-export --out=analysis/data/live.jsonl` (`DATABASE_URL` set; `--source`, `--mode` and `--patch` narrow it, `crates/server/README.md`) |
| An AI development run on this checkout | `cargo jackioh stats --games 200 --out analysis/data/dev.jsonl` (R378) |
| The training lanes' games | `~/training-out/<lane>/*.jsonl` on the training box (`training/README.md`) |

`analysis/data/` is git-ignored: records are data, and a live game's id is a match id. Only the fixture below is
committed.

## The loader

```python
from jackioh_records import card_stats, load, load_catalog

records = load(["analysis/data/live.jsonl", "analysis/data/dev.jsonl"])
records.games  # one row per record: id, source, mode, patch, first, winner, reason, turns
records.seats  # game × seat: pilot, went_first, result (win, draw or loss)
records.seat_cards  # game × seat × distinct card in the deck: in_opening, drawn, played, first_played_turn
stats = card_stats(records, source="all", mode="random", patch="v0.3.1", pilot="ai")
catalog = load_catalog()  # id → name, set, rarity, cost, type, tags, off crates/cards/catalog.json
```

`card_stats` is R377 and takes `stats-cards`' flags with its defaults: live games unless `source` is `dev` or
`all` (R378), every mode and patch unless named, every seat unless `pilot` is `human` or `ai`. Each deck counts
once per game, a draw is a game not won, and the played delta is the played win rate minus the drawn-but-not-
played win rate in percentage points. It returns one row per card with `<breakdown>_games`, `_wins`, `_draws`
and `_rate` for `in_deck`, `opening_hand`, `going_first`, `going_second`, `played` and `drawn_not_played`, and
`played_delta`; the frame's `attrs` hold `games` and `decks`.

`load` reads the fields it names and ignores the rest, names a bad line by file and line, and refuses a record id
it has already read (`stats report` would count the game twice), so files that overlap are trimmed first.

Two more loaders read what is not a record file:

```python
from jackioh_records import load_generations, load_sweep

sweep = load_sweep(["sweep-1.jsonl", "sweep-2.jsonl"])  # cargo jackioh sweep --json's lines
generations = load_generations(".")  # a repository root
```

`load_sweep` is one row per card and tier (`card`, `tier`, `games`, `drawn_games`, `affordable_turns`, `plays`,
`errors`, `timeouts`, `eval_delta_sum`, `eval_delta_count`, `eval_delta_mean` (NaN with no plays), `flags` (a list
of `error`, `timeout`, `neverPlayed`, `selfHarm`) and `unswept`). It skips a pass-2 line, so the files of a whole
sweep may be named together, and refuses a card and tier it has already read. `load_generations` is one row per
generation, oldest first, from `training/history/*.jsonl` and `crates/ai/generation.json` under the root (the
history's line for a generation wins over `generation.json`'s, which holds the newest again and generation 0);
`vs_random_wins` and `vs_random_games`, `vs_parent_wins` and `vs_parent_games` are the gate's `"93/100"` split in two.

## The notebooks

`analysis/notebooks/`, opened from the repository root with `uv run --project analysis jupyter lab`. They are
committed without outputs (`jupyter nbconvert --clear-output --inplace analysis/notebooks/*.ipynb` after editing
one; `test_notebooks.py` fails on a notebook that holds any), and `test_notebooks.py` runs each on the fixtures.

| Notebook | What |
|---|---|
| `01-card-win-rates.ipynb` | R377's breakdowns for every card beside their games, by set and rarity, the 15 best and worst cards by in-deck win rate and by played delta, one patch against the one before it, and a development run against live games of the same patch (R378) |
| `02-decks-and-play.ipynb` | Mana curves by mode and pilot, play rate and the cards most often drawn and not played, the turn cards are first played on, and the first player's win rate and the length of a game by mode and patch |
| `03-ai-sweep-and-generations.ipynb` | A shadow-ban sweep's pass-1 lines per card and tier with its flags, and the AI's generations against random play and their parents, per lane |

Each one's first cell reads its inputs from the environment and defaults to the fixtures, saying so when it does:

| Variable | Read by | What | Default |
|---|---|---|---|
| `JACKIOH_RECORDS` | 01, 02 | record files, comma-separated | `fixtures/dev-40.jsonl`, `fixtures/live-40.jsonl` |
| `JACKIOH_CATALOG` | all | the catalog | `crates/cards/catalog.json` |
| `JACKIOH_SWEEP` | 03 | pass-1 sweep lines, comma-separated files | `fixtures/sweep-small.jsonl` |
| `JACKIOH_REPO` | 03 | a repository root, for `crates/ai/generation.json` and `training/history/` | `fixtures/repo` |
| `JACKIOH_SOURCE` | 01, 02 | `live`, `dev` or `all`: the games the figures read (R378: live unless asked) | `live` |
| `JACKIOH_PATCH` | 01, 02 | the patch to read | the newest patch among the games read |
| `JACKIOH_MIN_SAMPLE` | 01, 02 | the games a card needs to be ranked | `CARD_STATS_MIN_SAMPLE`, read off `crates/server/src/config.rs` |

Set them where Jupyter starts, so the kernel inherits them. A relative path is read from the repository root, wherever the notebook is opened (run the commands below from there). A card with fewer games than the minimum is shown,
marked `*`, and never ranked, as on the statistics page (R654); the fixtures are 80 games, so little of what they
draw reaches the minimum of 20 (`test_notebooks.py` also runs 01 and 02 with `JACKIOH_MIN_SAMPLE=3`).

```
# Live games off the database and a development run: files of both sources may be named together (R378), and the notebooks read live games unless JACKIOH_SOURCE says dev or all, and the live-against-development section reads them apart
jackioh-server stats-export --out=analysis/data/live.jsonl
cargo jackioh stats --games 200 --out analysis/data/dev.jsonl
JACKIOH_RECORDS=analysis/data/live.jsonl,analysis/data/dev.jsonl uv run --project analysis jupyter lab

# A development run of this checkout alone
JACKIOH_SOURCE=dev JACKIOH_RECORDS=analysis/data/dev.jsonl uv run --project analysis jupyter lab

# The training lanes' games, copied off the training box (training/README.md), and its checkout's history
JACKIOH_SOURCE=dev JACKIOH_RECORDS="$(ls analysis/data/training/*.jsonl | paste -sd, -)" uv run --project analysis jupyter lab
JACKIOH_REPO=/path/to/the/training/checkout JACKIOH_SWEEP=sweep-1.jsonl,sweep-2.jsonl uv run --project analysis jupyter lab
```

A notebook that finds nothing to compare says so (one patch only, no live games, no card with enough games)
rather than drawing an empty chart. Files that overlap are refused by the loader (a record id it has read), so a
record named twice is trimmed first.

## Holding it to the Rust

`cargo jackioh stats report <file>… [--source] [--mode] [--patch] [--pilot] [--card] [--json]` prints the same
figures from `jackioh_engine::card_stats`, with no database. `tests/test_card_stats.py` checks the loader's
`card_stats` against that report over a committed fixture: wins and games exactly, rates to 1e-9.

| File | What |
|---|---|
| `fixtures/dev-40.jsonl` | 40 AI development games (R378), as `cargo jackioh stats` writes them |
| `fixtures/dev-40.report.json` | `stats report` over them, as JSON |
| `fixtures/live-40.jsonl` | 40 more AI games, filed as live ones: `make_live.py` gives them live ids, patches v0.3.0 (16) and v0.3.1 (24), the modes bo1 and bo3 and a human first seat. Not played by anyone: the notebooks' live and patch comparisons need records of that shape |
| `fixtures/sweep-small.jsonl` | `cargo jackioh sweep --json` over four cards (pass 1 at Easy and Hard), as it wrote it |
| `fixtures/repo/` | The files `load_generations` reads under a repository root: `crates/ai/generation.json` and `training/history/*.jsonl`. **Made up**: three promotions on two lanes, so the charts have a line. The real histories are empty until the first promotion |

To remake them (after a change to the record's shape or to R377's arithmetic, never to quiet a failing test):

```
cargo jackioh stats --games 40 --series analysis-fixture --out analysis/fixtures/dev-40.jsonl
cargo jackioh stats report analysis/fixtures/dev-40.jsonl --source=all --json > analysis/fixtures/dev-40.report.json
cargo jackioh stats --games 40 --patch v0.3.0 --series analysis-fixture-prev --out /tmp/prev-40.jsonl
python3 analysis/fixtures/make_live.py /tmp/prev-40.jsonl > analysis/fixtures/live-40.jsonl
cargo jackioh sweep --json core-002 core-011 classic-020 classicplus-005 > analysis/fixtures/sweep-small.jsonl   # about 70 seconds
```

The fixture is a snapshot: it is a file, so a later change to the engine, the cards or the AI leaves it as it is,
and it is remade only for the reasons above.
