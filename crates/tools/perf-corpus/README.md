# The perf corpus (#4924)

`cargo jackioh perf` (#3429, `../src/perf.rs`) times the engine on the heaviest states of a set of seeded games. It used to deal
each game from the live pools on every run (`spec_for_seed` in `../src/golden.rs` shuffles the shipped `FUZZ_POOL` and the handicap
pool, `duos_fuzz.rs` the co-op decks), so the day the next card landed every game was dealt again and the medians moved for reasons
the change under test had nothing to do with. Each file here pins one game's deal: its decks and its other start inputs, kept
literally. `perf` plays every game from its record, never from a live pool, so the same build always times the same games.

## A record

One JSON file per game, named `<kind>-<seed>-<catalogVersion>.json` (one file each, so two changes never conflict); `perf --pin`
writes `…-2.json`, `…-3.json` and on when the name is taken.

| Field | Holds |
| --- | --- |
| `kind` | `plain`, `handicapped`, `run-start` or `coop`. |
| `seed` | Which game of its kind. A `plain` game is golden seed `PERF_SEED_FIRST`–`PERF_SEED_LAST` and a `handicapped` one golden seed `PERF_HANDICAP_SEED_FIRST`–`PERF_HANDICAP_SEED_LAST`, whose seed names its game's, policy's and mulligan order's streams as `spec_for_seed` names them (`jackioh-fuzz-<seed>`, `jackioh-fuzz-handicap-<seed>`). The `coop` game is `PERF_COOP_SEED`, which names its streams (`perf-duos-<seed>`). The `run-start` game is `PERF_RUN_START_SEED`, a label: its streams are `perf-run-start`'s own. |
| `catalogVersion` | What `cargo jackioh catalog-version` printed the day it was pinned; a version in `crates/cards/patches/patches.json`. |
| `decks` | Every kind but `coop`: p1's deck then p2's. |
| `duosDecks` | `coop` alone: the four seats' decks, `a1`, `a2`, `b1` and `b2`. |
| `handicaps` | `handicapped` and `run-start` alone: each handicapped seat's handicap (R180). |
| `deckMarks` | `run-start` alone: each seat's deck marks (R2702000). |
| `retired` | Optional: which ids left the catalog, and in which patch. |

Only a kind's own fields may be present, and each of them must be. The corpus plays exactly the games the `PERF_*` constants name,
each from its one live record: a game with no live record, a second live record of one game, and a live record of a game no constant
names each fail the run, naming the file and what to do. The deals are pinned, not the logs: each game is played again from its
record under its fixed streams on every run, so an engine change still moves the timings.

## Adding a record

Only `cargo jackioh perf --pin` writes one. It deals each game the `PERF_*` constants name that has no live record from the shipped
pools, as `perf` dealt it live before #4924, and writes it under the first free name. It never rewrites a file, and it writes nothing
while the corpus has a problem. Widening a range (`PERF_SEED_LAST`, say) is the constant's change and `perf --pin` in the same change;
narrowing one leaves live records of games no constant names, which `perf` fails until they are deleted.

## Retiring a record

When a patch removes or renames an id a record holds, `perf` fails it, naming the id and the marker to add. Add
`"retired": "<which ids, which patch>"` to the record, then run `cargo jackioh perf --pin` in the same change, which deals the game
again into a new file. A retired record is listed in `--json`'s `corpus.retired` and not played. Retirement excuses only the missing
ids: the record's shape still lints. Never edit its decks or other pinned inputs, and never delete the file: the record is what that
game was.

## Running it

```
cargo jackioh perf --smoke --json
cargo test -p jackioh-tools perf
```

`perf` reads and lints every record first, and fails on any problem before it times anything. `--json`'s `corpus.games` names the
records played, in play order. The tools' `cargo test` runs `tests/perf.rs`'s `perf_smoke_times_the_corpus_once` on every pull
request, which holds `corpus.games` and `corpus.retired` to the files here, and `perf.rs`'s unit tests, which hold the checked-in
corpus to the games the constants name.

## Scope

The deals only. A card that draws from an in-game random pool (a Discover, a random summon) draws from the live catalog, so a game
can still play differently once that pool grows: the pinned decks keep the corpus close, not frozen. A new card joins no pinned game
until a record is dealt again.
