# The fuzz regression corpus (#1197)

A fuzz failure is reported as a seed, and a seed deals its decks by one shuffle of the live `FUZZ_POOL` (`decks_for_seed` in
`../src/fuzz.rs`). It also manufactures boards from the live `BOARD_POOL`. The day the next card lands either pool can grow, so an old
seed can redraw another game. Each file here keeps one failure with the decks, literal start inputs and endings-channel choice it used,
and `cargo jackioh fuzz --regress` plays that exact setup again, never a deal or board from a live pool.

## A record

One JSON file per record, named `seed-<seed>-<catalogVersion>.json` (one file each, so two changes never conflict):

| Field | Holds |
| --- | --- |
| `seed` | Names the game's and the policy's streams (`jackioh-fuzz-<seed>`); 1 or more. |
| `catalogVersion` | What `cargo jackioh catalog-version` printed on the day it failed; a version in `crates/cards/patches/patches.json`. |
| `decks` | p1's deck then p2's, as the report's `seed N p1 deck:` / `p2 deck:` lines print them. An ordinary record holds `DECK_SIZE` (25) distinct ids each; one found before #1705 may hold 20 each (R1705003). A `run` record holds each deck independently between `RUN_START_DECK_MIN` and `RUN_START_DECK_MAX`. |
| `start` | Optional: `plain`, `dealt`, `last-boards`, `glitch-boards` or `run`. Omitted means `plain`, preserving every legacy record. |
| `dealt` | Required only for `dealt`: `["p1"]`, `["p2"]` or `["p1", "p2"]` (R433). |
| `lastBoards` | Required only for `last-boards`: p1's literal entries then p2's, each entry `{ "defId", "radiant" }` (R417, R564). |
| `glitchBoards` | Required only for `glitch-boards`: the two literal boards in the same shape (R678). |
| `handicaps` | Required only for `run`: an explicit dungeon-fight handicap for each seat (R180, R2922000, R2922001). |
| `deckMarks` | Required only for `run`: an explicit per-seat list of `{ "index", "radiant", "upgrades" }` (R2702000); use an empty list when a seat has no marks. |
| `endings` | Optional boolean: whether the seed took the endings channel. Omitted means `false`, preserving every legacy record. |
| `found` | Which run failed, how, and the issue that fixed or filed it. |
| `retired` | Optional: which ids left the catalog, and in which patch. |

Only the fields belonging to `start` may be present. The report prints them as JSON lines ready to copy beside the decks. It prints no
log: the seed, decks, literal setup inputs and endings choice rebuild the game (§9.3).

## Adding a record

In the same change that fixes or files a failure, copy the report's JSON `start`, applicable start-input and `endings` lines, its
`seed N p1 deck:` and `p2 deck:` lines, and the output of `cargo jackioh catalog-version` into `seed-<n>-<version>.json`. Run
`cargo jackioh fuzz --regress`: a record must pass once its bug is fixed. A record of a bug that is only filed stays out until the fix,
which adds it.

## Retiring a record

When a patch removes or renames an id a record holds, the lint fails it. This includes a board id and every ingredient, recursively, of
a fused board id that is no longer rebuildable. Add `"retired": "<which ids, which patch>"` to the record. A retired record is listed
and not replayed. Retirement excuses only those missing catalog ids: start-field consistency, deck sizes, handicaps and marks still lint.
Never edit its decks, boards or other pinned inputs, and never delete the file: the record is what the failure was.

## Running it

```
cargo jackioh fuzz --regress
cargo test -p jackioh-tools fuzz_regression_corpus
```

`--regress` lints every record first: its start-specific fields, deck sizes, dealt seats, handicaps and marks, and its deck and board ids
against the live catalog with every set previewed (R1420). Then it creates, fully folds and checkpoint-folds the same pinned options and
runs the gate's checks: no panic, a real ending inside the cap, every invariant, and a log that folds back to the live hash. The tools'
`cargo test` runs the same through
`the_checked_in_corpus_lints_clean_and_every_record_replays`, but that test is `#[ignore = "daily: …"]` (#4093) and runs in the daily
tests, so a pull request does not replay the corpus; the daily run does.

## Scope

The unhandicapped gate's 1v1 games only, with their pinned plain, dealt, board or dungeon-fight start. The handicap-prefixed wave and
Duos deal other decks (`handicap_decks_for_seed`, `duos_fuzz.rs`) and have no corpus.
