# The fuzz regression corpus (#1197)

A fuzz failure is reported as a seed, and a seed deals its decks by one shuffle of the live `FUZZ_POOL` (`decks_for_seed` in
`../src/fuzz.rs`). The day the next card lands the pool grows and every past seed redeals other decks, so an old report's seed stops
reproducing its game. Each file here keeps one failure with the decks it was dealt, and `cargo jackioh fuzz --regress` plays it again
with those decks, never a deal from the live pool.

## A record

One JSON file per record, named `seed-<seed>-<catalogVersion>.json` (one file each, so two changes never conflict):

| Field | Holds |
| --- | --- |
| `seed` | Names the game's and the policy's streams (`jackioh-fuzz-<seed>`); 1 or more. |
| `catalogVersion` | What `cargo jackioh catalog-version` printed on the day it failed; a version in `crates/cards/patches/patches.json`. |
| `decks` | p1's deck then p2's, 20 distinct ids each, as the report's `seed N p1 deck:` / `p2 deck:` lines print them. |
| `found` | Which run failed, how, and the issue that fixed or filed it. |
| `retired` | Optional: which ids left the catalog, and in which patch. |

The report prints no log, so a record keeps none: the seed and the decks rebuild the game (§9.3).

## Adding a record

In the same change that fixes or files a failure, copy the report's `seed N p1 deck:` and `p2 deck:` lines and the output of
`cargo jackioh catalog-version` into `seed-<n>-<version>.json`. Run `cargo jackioh fuzz --regress`: a record must pass once its bug is
fixed. A record of a bug that is only filed stays out until the fix, which adds it.

## Retiring a record

When a patch removes or renames an id a record holds, the lint fails it. Add `"retired": "<which ids, which patch>"` to the record. A
retired record is listed and not replayed. Never edit its decks or delete the file: the record is what the failure was.

## Running it

```
cargo jackioh fuzz --regress
cargo test -p jackioh-tools fuzz_regression_corpus
```

`--regress` lints every record first (its shape, its version, its ids against the live catalog with every set previewed, R1420), then
plays each one with the gate's checks: no panic, a real ending inside the cap, every invariant, and a log that folds back to the live
hash. The tools' `cargo test` runs the same, so the `rust` check does.

## Scope

The gate's 1v1 deal only. The handicapped wave and Duos deal other decks (`handicap_decks_for_seed`, `duos_fuzz.rs`) and have no corpus.
