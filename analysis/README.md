# analysis

Python over JackiOh's game records (SPEC §9.11, R376–R378): a loader that turns record files into pandas
tables, R377's card win rates computed over them, and the project the notebooks run in. Nothing in `crates/`,
`apps/`, `e2e/` or `spec/` reads this directory, and nothing here changes a rule, the wire, the database or
what a player sees: it reads records and prints figures.

```
uv sync --frozen --project analysis                    # Python 3.13 and the pinned dependencies, into analysis/.venv
uv run --project analysis pytest analysis/tests        # the loader's tests; the card figures equal the Rust report's
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
records.games        # one row per record: id, source, mode, patch, first, winner, reason, turns
records.seats        # game × seat: pilot, went_first, result (win, draw or loss)
records.seat_cards   # game × seat × distinct card in the deck: in_opening, drawn, played, first_played_turn
stats = card_stats(records, source="all", mode="random", patch="v0.3.1", pilot="ai")
catalog = load_catalog()      # id → name, set, rarity, cost, type, tags, off crates/cards/catalog.json
```

`card_stats` is R377 and takes `stats-cards`' flags with its defaults: live games unless `source` is `dev` or
`all` (R378), every mode and patch unless named, every seat unless `pilot` is `human` or `ai`. Each deck counts
once per game, a draw is a game not won, and the played delta is the played win rate minus the drawn-but-not-
played win rate in percentage points. It returns one row per card with `<breakdown>_games`, `_wins`, `_draws`
and `_rate` for `in_deck`, `opening_hand`, `going_first`, `going_second`, `played` and `drawn_not_played`, and
`played_delta`; the frame's `attrs` hold `games` and `decks`.

`load` reads the fields it names and ignores the rest, names a bad line by file and line, and refuses a record id
it has already read (`stats report` would count the game twice), so files that overlap are trimmed first.

## Holding it to the Rust

`cargo jackioh stats report <file>… [--source] [--mode] [--patch] [--pilot] [--card] [--json]` prints the same
figures from `jackioh_engine::card_stats`, with no database. `tests/test_card_stats.py` checks the loader's
`card_stats` against that report over a committed fixture: wins and games exactly, rates to 1e-9.

| File | What |
|---|---|
| `fixtures/dev-40.jsonl` | 40 AI development games (R378), as `cargo jackioh stats` writes them |
| `fixtures/dev-40.report.json` | `stats report` over them, as JSON |

To remake them (after a change to the record's shape or to R377's arithmetic, never to quiet a failing test):

```
cargo jackioh stats --games 40 --series analysis-fixture --out analysis/fixtures/dev-40.jsonl
cargo jackioh stats report analysis/fixtures/dev-40.jsonl --source=all --json > analysis/fixtures/dev-40.report.json
```

The fixture is a snapshot: it is a file, so a later change to the engine, the cards or the AI leaves it as it is,
and it is remade only for the reasons above.
