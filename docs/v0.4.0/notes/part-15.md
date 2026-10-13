# v0.4.0 Part 15: cards to Luau, batch 7 (triggers)

Notes for issue #457 (v0.4.0 part 15 of 29, a fullsend part of #442): the eighteen trigger cards of
batch 7, ported by `docs/v0.4.0/PORTING.md`. Twelve now run from a `.luau` file; six stay in Rust
for a binding the API does not have (PORTING.md §5, reason 1). Base commit: `c65d6d55c`
(`bot/issue-442` with parts 6 and 7, every card of this batch still Rust).

## 1. Ported to Luau

Each `.luau` sits beside its `.rs`, which keeps its header, `pub const ID`, its comments and its
`#[cfg(test)] mod tests`, with `script()`, its helpers and its `use` lines removed. Every trigger's
`on` is data; its `when` and `run` are functions; every effect is in the Rust order; trigger ids are
the Rust ones.

| Card | Id | What it declares |
|---|---|---|
| Core #6 Mana Well | `core-006` | `startOfTurn`: `gain_mana` of `param("mana")`; `radiant = "base"` |
| Core #33 Unstable Clone Machine | `core-033` | a `cardResolved` trigger (no `when`, as in Rust) per face: `shuffle_into` copies, the played flag kept on base, all Radiant on radiant |
| Classic #5 Tesla | `classic-005` | a `summoned` / `cardResolved` trigger with `when`: `damage`, then `animate` in DEF; `radiant = "base"` |
| Classic #17 Counterspell | `classic-017` | a `cardAnnounced` trigger: `counter_play`; radiant adds the copy (`add_to_hand`, face and language kept) |
| Classic #38 Jackiestan Auctioneer | `classic-038` | a `cardPlayed` trigger: the reveal (`remember`, `animate`) or the sale (`draw`, `damage`, `animate`); `radiant = "base"` |
| Classic #53 Plague Crawler | `classic-053` | `targets`, a `cry` (`place_plague`) and a `counterChanged` trigger (`draw`); `radiant = "base"` |
| Classic #58 Common Resources | `classic-058` | `startOfTurn` (radiant also `endOfTurn`): `param("cards")` separate `draw_from_opponent`s |
| Classic #63 Crop Dusting | `classic-063` | a `turnStarted` trigger: `place_plague_each`, then `draw`; `radiant = "base"` |
| Classic #64 Malzahar's Recycler | `classic-064` | `endOfTurn` (`discard_random` 2) and a `discarded` trigger (`draw` 1, radiant the deck) |
| Classic #72 Grand Counterspell | `classic-072` | a `cardAnnounced` trigger: `counter_play` to the graveyard, radiant to the thief |
| Classic+ #1 Doom Shroom | `classicplus-001` | an `attackDeclared` trigger: `exile_all`, then `lock_own_zone` |
| T-AI-9 Refusal | `classicplus-t-ai-09` | a `cardAnnounced` trigger: `counter_play`, radiant then `draw` 1 |

A reader that answers Rust's `None` (`ctx:find_instance`, `ctx:self_`, `ctx:attack_target_of`)
hands Luau mlua's `null`, a light userdata, which is truthy and cannot cross back. Every such read
is tested with `type(x) == "table"`, never truthiness or `~= nil`. An event's optional field
(`radiant`, `exitsFrom`, `placed`) is left off the wire when absent, so it reads as `nil`.

Two reads are rearranged where every read involved is pure and nothing between them draws or
writes, so no answer changes: Doom Shroom reads the attacker before the attack's target (Rust reads
the target first), and Refusal's base face reads the controller's Units once per announce rather
than once per declared target.

## 2. Kept in Rust

Each keeps its `.rs` exactly as the base commit has it and has no `.luau`.

| Card | Id | Why |
|---|---|---|
| Core #41 Sheepish | `core-041` | its `when` reads `left_field_since_resolved(state, event)`, which has no Luau form (already in `api-gaps.md`) |
| Classic #10 Exile | `classic-010` | the radiant face's `for_each_card` picks from a budget its `run` read off the announce (`threshold` minus `costPaid`); the bound `for_each_card` calls a face function with `ctx` only, no data, and drawing the picks in `run` instead would move those rng draws ahead of the counter, which the Rust draws after it |
| Classic #65 Ace in the Hole | `classic-065` | its `endOfTurn` remembers `state.turn` on heads and its trap's `when` compares it to `state.turn`; no reader answers the turn number. (The Lucky coin is expressible as repeated `ctx:rng_chance(1, 2)`, so that is not the gap.) |
| Classic+ #2 Groom Shroom | `classicplus-002` | its `for_each_card` grants Taunt in the lanes `fill_board_zones` named before the summons filled them; the bound `for_each_card` takes no data, and once the summons ran no reader recovers which lanes they were |
| Classic+ #12.8 Frostspatula | `classicplus-012-8` | its `death` hook reads this action's events (`ctx.events`) for kills in its own death's pass; no reader answers the action's events |
| T-AI-8 Rate Limit | `classicplus-t-ai-08` | its `when` compares `state.active` with the player who played; no reader on an effect context answers whose turn it is |

These four new gaps, which five of the cards wait on, belong in `docs/v0.4.0/api-gaps.md`: a data
argument for `for_each_card`'s `cards` function (Exile, Groom Shroom), and readers for the turn
number, the active player and the action's events. This part does not edit that file (the plan
names only the card files and these notes, and nineteen batches build side by side); #442's
reconcile folds them in.

## 3. The tests

No test asserts anything new or anything less: the six Rust cards' files are untouched, and in the
twelve ported files (199 tests) two kinds of line changed, both because `script()` and the
module's `use` lines are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()` (Tesla, Counterspell, Auctioneer,
  Plague Crawler, Common Resources, Crop Dusting, Recycler, Grand Counterspell, Doom Shroom,
  Refusal's `super::super::script()` with `super::super::ID`), and `use super::{script, ID};` is
  `use super::ID;`. `scripts_of()` is the registry `build.rs` generates, so the test reads the
  Luau card exactly as the engine registers it.
- Tesla's test module gains `use std::sync::Arc;`: its "the radiant face is the very same script"
  check (`Arc::ptr_eq` of the two faces' `run`) reached `Arc` through the removed
  `use jackioh_engine::prelude::*`. The check still holds: `radiant = "base"` clones the base
  `Script`, so both faces share each `Arc`.

## 4. Proofs

All run on this build, in this worktree, on linux x86_64, 4 threads, AMD EPYC 7763.

- **Card tests.** `cargo test -p jackioh-cards --lib -- c006_mana_well c033_unstable_clone_machine
  c005_tesla c017_counterspell c038_jackiestan_auctioneer c053_plague_crawler c058_common_resources
  c063_crop_dusting c064_malzahars_recycler c072_grand_counterspell c001_doom_shroom
  t_ai_09_refusal`: 199 passed, 0 failed.
- **`luau diff`**, parent `jackioh` built from the base commit:
  - the eleven deckable ported cards, plus Classic+ #43 AI Slop and #78 Claude's Datacenter,
    whose random AI-token pools are what bring T-AI-9 into a game (no card's `refs` name it, so
    it cannot be forced): 200 games, seeds 1–200, 14,850 steps, **0 mismatches**;
  - every forceable card of the batch (the above plus Sheepish, Exile, Ace in the Hole and Groom
    Shroom, which stay Rust but meet the ported cards in play): 200 games, 13,070 steps,
    **0 mismatches**.
- **What those games ran.** The first run's games, re-recorded with `golden record --seeds 1..200`
  and the same forced list, then folded by a throwaway counter outside the repository: plays /
  games with the card / trap firings — Mana Well 181 / 132, Clone Machine 225 / 151, Tesla
  226 / 140 / 541, Counterspell 222 / 144 / 171, Auctioneer 241 / 163 / 1,518, Plague Crawler
  437 / 189 (132 placements on a Crawler), Common Resources 234 / 147, Crop Dusting
  239 / 152 / 223, Recycler 236 / 154, Grand Counterspell 235 / 151 / 222, Doom Shroom
  180 / 121 / 69, Refusal 64 / 48 / 5.
- **`golden check`**: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- **`fuzz --seeds 200`**: 200 passed, 0 failed; 0 panics, 0 over the bound, 0 replay mismatches,
  0 invariant violations.
- **`luau bench --parent-bin`**: within every budget.

| Number | Parent (base) | This build | Ratio | Budget |
|---|---|---|---|---|
| `fuzz --seeds 200` wall time | 11,857.1 ms | 12,743.7 ms | ×1.07 | ≤ 2× |
| Slowest `gate` decision | 248.7 ms | 241.7 ms | ×0.97 | ≤ 1.5× and < 1,500 ms |
| Golden step | 1.846 ms | 1.902 ms | ×1.03 | ≤ 2× |

- **`luau-analyze --mode=strict`: not run.** There is no Luau toolchain on this box and fetching
  the 0.740 release was refused, so strict analysis of the twelve files is left to a run that has
  it. `build.rs` lints and compiles every one (`jackioh_luau::compile`), and each loads and runs
  in its card's tests. Every hook context and trigger event is annotated (`J.EffectContext`,
  `J.GameEvent`), every union-typed verb argument is a hoisted, annotated local
  (`J.TargetSpec`, `J.AddToHandArgs`), and events are narrowed on `event.type` before a field is
  read.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo jackioh catalog check`, `patches check` and `spec check` (805 rulings, 1710 proofs): clean.
- **`cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`**: the cards'
  unit tests 5,860 passed, the cross-card tests 752 passed (`hidden_information.rs`'s R119 case with
  #33 among them), the AI's 325 passed (the redaction and determinization cases with Counterspell and
  Doom Shroom face-down among them), the engine's rules 2,637 and golden 19, the server's 1,268, the
  Luau host's 48; `apps/web/src/wire/` unchanged. Three tests of `jackioh-tools` fail, and fail the
  same way on the base commit (run there in a scratch worktree, since removed): the surface tests
  `luau_core_002_bigot` and `luau_reports_print_as_markdown_and_json` (part 7's notes report them),
  and `acceptance.rs`'s `v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other`,
  which names `crates/cards/Cargo.toml`'s `jackioh-luau` dependency from part 4. This part touches
  no manifest and no tool; they are the reconcile's.

## 5. Decisions the plan did not settle

- **The parent was built in this worktree from the untouched base**, not in a second worktree:
  the same commit, one full release build saved. The build started before any file changed; its
  `build.rs` ran (and wrote a registry with only the three pilots as Luau, every other card naming
  its Rust `script()`) a minute before the first card file changed, and the compile after it
  succeeded, which it could not have done against a `.rs` whose `script()` was gone. The binary was
  copied to a path outside the worktree before the next build. Rejected: `git worktree add ../base`
  and a second full release build of the same tree.
- **`script()` in a test reads `crate::scripts_of()`**, the crate's generated registry, not
  `jackioh_engine::scripts::registered_entry`, which answers only after `register_all()` and would
  have the tests depend on registration order. Rejected: a shared test helper in `lib.rs`, which
  the plan does not name and every parallel batch would add too.
- **T-AI-9 reaches `luau diff` through its pools.** Rejected: dropping it from the diff, since it
  then would never be played on both builds.
- **New API gaps are written here, not in `api-gaps.md`** (section 2).
