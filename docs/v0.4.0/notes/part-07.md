# v0.4.0 Part 7: the proof tools, the baseline and the porting guide

Notes for issue #449 (v0.4.0 part 7 of 29): `luau diff` / `luau bench` /
`golden record` / `golden check --file` as the batches will use them,
`docs/v0.4.0/perf.json` measured on `main` at the branch point, and
`docs/v0.4.0/PORTING.md` checked against part 6's landed API.

## 1. What changed

- Groundwork #615 had already landed the mechanics on `main`
  (`crates/tools/src/luau.rs`, `golden record` / `golden check --file` in
  `crates/tools/src/golden.rs`, the README rows, the integration tests), so
  this part adds no new `src`: the commands already do what the plan asks
  (200 forced games by default with §10.7's random policy, tokens forced
  through the deckable card whose `refs` name them per R279, the
  golden-trace format with state, both views, events and the legal set,
  parent replay as oracle, seed/step/which-hash/action on mismatch, exit 1;
  bench with `BENCH_BUDGETS` 1.5× / 2× / 2× plus the absolute
  `AI_GATE.max_decision_ms` cap). Verified, not rebuilt.
- `crates/tools/tests/luau.rs`: new test
  `luau_diff_with_the_pilot_cards_forced_reports_0_mismatches` (the three
  Luau pilots forced, self-parent, 0 mismatches), beside the existing
  forced-deck, self-parent and doctored-hash tests.
- `docs/v0.4.0/perf.json`: the baseline, three numbers on `main` at the
  branch point `f01a13d5d` (all cards Rust) with machine and date.
- `docs/v0.4.0/PORTING.md`: the batch procedure, the translation rules, the
  three pilots plus three part-6 fixtures each beside their Rust, the
  failure guide, and when a card stays Rust.

## 2. How the baseline was taken

Branch point `f01a13d5d` (`git merge-base origin/bot/issue-442 main`):
`git worktree add ../base f01a13d5d`, then release builds on both sides
(each ~8–9 minutes on the 2-thread bot box), then one
`luau bench --parent-bin ../base/target/release/jackioh --record` run, so
parent and current share the same machine and load. `perf.json` keeps the
parent side; the full record (both sides, budgets, pass) is below.

| Number | `main` at `f01a13d5d` (parent) | This branch (current) | Budget | Verdict |
|---|---|---|---|---|
| `fuzz --seeds 200` wall time | 20,094 ms | 21,638 ms (×1.08) | ≤ 2× parent | PASS |
| Slowest `gate` perf decision (scaled) | 306.5 ms | 227.5 ms (×0.74) | ≤ 1.5× parent and < 1500 ms | PASS |
| Golden step, `RAYON_NUM_THREADS=1` | 1.708 ms | 1.766 ms (×1.03) | ≤ 2× parent | PASS |

Machine: linux x86_64, 2 threads, Intel Xeon Platinum 8488C. Date:
2026-10-13. The worktree was removed after the run; only this worktree
delivers.

`luau diff --parent-bin ../base/target/release/jackioh --cards
core-001,core-002,core-003` (the pilots, defaults: 200 games, seeds 1–200):
200 games, 21,181 steps, 0 mismatches. The hotseat fixture folds to
`5c3d578c` in the same run.

## 3. Decisions the plan did not settle

- **The gate compares two binaries live; `perf.json` is a witness, not the
  comparator.** The plan says each number goes "against `perf.json`", but
  #615's landed (and tested) design measures parent and current on the same
  machine in the same run, which removes hardware noise a committed file
  cannot. Rejected: a `--baseline perf.json` mode (extra scope, breaks the
  tested CLI). `perf.json` records the branch-point numbers for context.
- **No `src` change.** The plan lists `luau.rs` as new and `golden.rs` /
  `main.rs` / README as changes, but #615 already built exactly that CLI on
  `main`, and this branch inherits it. Re-reading the code against the plan
  showed no gap, so the deliverable is tests + docs + baseline. Rejected:
  cosmetic refactors to touch the named files anyway.
- **PORTING.md quotes the pilots' pre-port Rust from the base commit.**
  The `.rs` files on this branch keep only `ID` plus tests; the Rust beside
  each Luau is what `build.rs` replaced (verified against `8d29cd5e7^`).

## 4. Provenance

`cargo test -p jackioh-tools --test luau`: 4 passed (forced decks, pilots,
self-parent, doctored-hash naming seed/step/hash with exit 1, traces kept).
`cargo test -p jackioh-tools --bin jackioh`: 163 passed, 2 failed —
`luau::surface::tests::luau_core_002_bigot` and
`luau_reports_print_as_markdown_and_json`, both pre-existing on this branch
(not touched by this part: part 4's pilot port removed `script()` from the
`.rs` sources #560's surface scanner reads; teaching surface about Luau
cards belongs to surface's owner / the reconcile, so reported, not fixed).
`cargo fmt --check` clean,
`cargo clippy -p jackioh-tools --all-targets -- -D warnings` clean,
`cargo jackioh spec check`: 805 rulings, 1710 proofs: ok.
`luau-analyze` was not run here (no Luau toolchain on this box); PORTING.md
documents the strict gate for the batches' own runs.
