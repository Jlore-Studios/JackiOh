# v0.4.0 Part 18: cards to Luau, batch 10 (prompts, modes and continuations, Core)

Notes for issue #460 (v0.4.0 part 18 of 29, a fullsend part of #442): the sixteen Core cards of
batch 10, ported by `docs/v0.4.0/PORTING.md`. Thirteen now run from a `.luau` file. Three stay in
Rust because the API is missing a binding (PORTING.md §5, reason 1). Base commit: `503395124`
(`bot/issue-442` with parts 6, 7, 9, 14 and 15; every card of this batch still Rust there).

## 1. Ported to Luau

Each `.luau` sits beside its `.rs`. The `.rs` keeps its `//!` header, `pub const ID`, the comment
above its tests and its `#[cfg(test)] mod tests`. Its `script()`, the helpers and constants only
`script()` used, and its `use` lines are gone. Every effect keeps the Rust order. Every `resume`
key, `step`, prompt text and option string is the Rust one, because a paused state and a replayed
log name them. Every declared number is still a `ctx:param` read (R386).

| Card | Id | What it declares |
|---|---|---|
| #7 Jewelosco Scarab | `core-007` | `cry`: `discover_from_catalog` (cost 2 / 3); `resume.chosen`: `add_to_hand` (radiant `costMod = -param("discount")`) |
| #17 Flood | `core-017` | base `cry`: `bounce_all` any; radiant `modes` (three options) + `cry`: the chosen mode, then `draw` of `param("draw")` |
| #24 Efficiency Dividend | `core-024` | `modes`, a `targets` entry with `forModes`, `cry` (X × `param` / `J.div(x, 2)`), `endOfTurn` (`live_self`, `was_played_this_turn`, `bounce`) |
| #30 Archivist | `core-030` | base `modes` + `cry` (`draw_from_library` of the extreme by `effective_cost`); radiant `cry`: `for_each_card` + `each_card` |
| #37 Gravedigger | `core-037` | base `startOfTurn`: `add_random_from_graveyard`; radiant `startOfTurn`: `discover_from_graveyard`, `resume.picked`: `bounce` then `set_cost_mod` |
| #39 Recycling Initiative | `core-039` | `cry`: `delay` (end of turn, `hook = "resume"`, `data.selfId`) then `exile`; `resume.copies`: one `add_to_hand` per other card played |
| #48 5pek Controller | `core-048` | base `cry`: `switch_all_positions` both; radiant `modes` + `cry` (enemy or both) |
| #52 Silly Silas | `core-052` | a `direction` mode + `cry`: `rotate`; `radiant = "base"` |
| #65 Masochism Mask | `core-065` | `staticFlags.quickdraw`, `startOfTurn`: `choose_mode`; `resume.firstPick` (radiant also `resume.secondPick`, the second prompt last in the list) |
| #72 Reminisce | `core-072` | `cry`: `discover_from_graveyard` then `exile` (parked); `resume.chosen`: `add_to_hand`, then `set_cost_mod` / `set_cost_override` |
| #82 KY's Trial | `core-082` | `cry`: `discover_from_catalog` (`set = "Core"`, `offer = "index"`); `resume.picked`: `ctx:def_by_index`, `add_to_hand` (radiant `costOverride = param("setCost")`) |
| #87 Pocket Chaos | `core-087` | `modes` (radiant adds gift / skip), `cry`: `swap({})`, the gift (`self_().costOverride`, else `ctx:printed_cost`), `exile` |
| #88 Twisting Nether | `core-088` | `cry`: `destroy_all` over both rows; radiant `modes` (enemy / all) |

Three translations are not one-to-one with the Rust. Each gives the same answers:

- **Archivist's radiant `for_each_card` passes its ids outright** (`cards = { … }`) rather than
  naming a `cards` function. The Rust closure reads the highest and lowest library cards when the
  list reaches the clause. The clause is the whole list, so nothing runs between the hook and the
  clause, and the hook reads the same library. A rebuild after a cast-on-draw prompt takes the ids
  from the part's memo (`effects/each.rs`), never from the source, so a source given outright
  resumes the same way. An empty library leaves `cards` out. The verb reads that as the empty set,
  as the Rust closure's empty `Vec` is (section 5 says why).
- **Recycling Initiative excludes its own play by `ctx:self_()`, not by `data.selfId`.** The Luau
  has no reader for `ctx.data` (section 2). The Cry still stores `data = { selfId = … }`, so the
  delayed entry in state is byte for byte the Rust one. The delay stores the same instance
  (`resume_self`'s `instance_id` is `ctx.self_.id`), so in the continuation `self_` is that card
  while it exists. Once it has ceased to exist, R86's `find_instance` skip drops its id from the
  copies anyway, so the copies come out the same either way.
- **Pocket Chaos reads its printed cost through `ctx:printed_cost(self.id)`.** That reader finds
  the live instance by id. The Rust passes `ctx.self_`. During its own Cry the card stands in
  `resolving`, so the two are the same card, and #87 is a fixed (4) with no cost hook.

A reader with no answer (`self_`, `live_self`, `find_instance`, `def_by_index`) hands Luau mlua's
`null`, a light userdata that is truthy. Every such read is tested with `type(x) == "table"`.

## 2. Kept in Rust

Each keeps its `.rs` exactly as the base commit has it and has no `.luau`.

| Card | Id | Why |
|---|---|---|
| #50 K-Pop Fanatic | `core-050` | its delayed `resume.steal` reads the chosen permanent's id from `ctx.data.targetId` (the Fanatic itself may be dead, R76), and no reader answers `ctx.data` |
| #51 KY's Private Tutor | `core-051` | its `resume.reveal` reads the type the bracket step carried in `ctx.data.type` (§10.6), and no reader answers `ctx.data`; by then the answer in `ctx.targets` is the bracket alone |
| #78 /fullsend | `core-078` | both its modifiers expire `{ until = "thisTurn", turn = ctx.state.turn }`, and no reader answers the turn number (the gap part 9 hit with #35 Lunar Eclipse) |

These belong in `docs/v0.4.0/api-gaps.md`: a `ctx:data()` reader (or the continuation's data
passed to a `resume` entry the way `after_state_check` passes it to `after_check`) for K-Pop Fanatic
and KY's Private Tutor, and the turn reader for /fullsend. Part 15 already lists the turn reader. This
part does not edit that file: the plan names only the card files and these notes, and nineteen
batches build side by side. #442's reconcile folds them in.

## 3. The tests

No test asserts anything new or anything less. The three Rust cards' files are untouched. In the
thirteen ported files (153 tests), three kinds of line changed, all because `script()` and the
module's `use` lines are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()`, the registry `build.rs` generates,
  so the test reads the Luau card as the engine registers it: Efficiency Dividend, Gravedigger,
  5pek Controller, Masochism Mask, Pocket Chaos (twice) and Twisting Nether (twice).
- `use super::script;` / `use super::{ID, script};` is `use super::ID;` (Efficiency Dividend,
  Gravedigger, 5pek Controller, Masochism Mask).
- KY's Trial's `use super::*;` now imports nothing its tests use (they name the card by their own
  `TRIAL` constant), so it is removed, as part 14 removed three such lines.

## 4. Proofs

All run on this build, in this worktree, on linux x86_64, 4 threads, Intel Xeon Platinum 8370C.

- **Card tests.** `cargo test -p jackioh-cards --lib -- c007_jewelosco_scarab c017_flood
  c024_efficiency_dividend c030_archivist c037_gravedigger c039_recycling_initiative
  c048_5pek_controller c052_silly_silas c065_masochism_mask c072_reminisce c082_kys_trial
  c087_pocket_chaos c088_twisting_nether`: 153 passed, 0 failed.
- **`luau diff`**, parent `jackioh` built from the base commit:
  - the thirteen ported cards forced: 200 games, seeds 1–200, 24,286 steps, **0 mismatches**;
  - all sixteen forced (the three Rust cards meet the ported ones in play): 200 games, 25,088 steps,
    **0 mismatches**.
- **What those games ran.** The first run's games, re-recorded with `golden record --seeds 1..200`
  and the same forced list, then folded by a throwaway counter outside the repository. Columns:
  resolutions on the base / radiant face (`cardResolved`), then the prompts the card opened and
  answered and the choices named in its plays.

  | Card | Resolved | Continuations and choices |
  |---|---|---|
  | Jewelosco Scarab | 333 / 4 | 338 Discovers opened and answered (`chosen`) |
  | Flood | 103 / 4 | radiant modes: 1 bounce all, 2 bounce enemy, 1 destroy enemy |
  | Efficiency Dividend | 2,661 / 164 | modes: 1,239 damage, 1,302 heal, 309 mana |
  | Archivist | 332 / 10 | modes: 173 highest, 159 lowest; 10 radiant `for_each_card` resolutions |
  | Gravedigger | 273 / 6 | 24 radiant Discovers opened and answered (`picked`) |
  | Recycling Initiative | 303 / 16 | 209 delayed `copies` entries ran or left |
  | 5pek Controller | 452 / 6 | radiant modes: 3 enemy, 3 all |
  | Silly Silas | 275 / 27 | directions: 150 left, 154 right |
  | Masochism Mask | 388 / 11 | 2,777 `firstPick` prompts opened and answered, 66 `secondPick` (it asks every start of turn on the field) |
  | Reminisce | 258 / 4 | 225 Discovers opened and answered (`chosen`) |
  | KY's Trial | 360 / 11 | 375 index Discovers opened and answered (`picked`) |
  | Pocket Chaos | 345 / 13 | swaps: 132 health, 108 board, 122 library; radiant: 4 gift, 9 skip |
  | Twisting Nether | 98 / 7 | radiant modes: 3 enemy, 2 all |

- **`golden check`**: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- **`fuzz --seeds 200`**: 200 passed, 0 failed; 0 panics, 0 over the bound, 0 replay mismatches,
  0 invariant violations.
- **`luau bench --parent-bin`**: within every budget.

| Number | Parent (base) | This build | Ratio | Budget |
|---|---|---|---|---|
| `fuzz --seeds 200` wall time | 13,091.608 ms | 12,843.015 ms | ×0.98 | ≤ 2× |
| Slowest `gate` decision | 254.201 ms | 259.598 ms | ×1.02 | ≤ 1.5× and < 1,500 ms |
| Golden step | 1.829 ms | 1.844 ms | ×1.01 | ≤ 2× |

- **`luau-analyze --mode=strict`: not run.** This box has no Luau toolchain. Fetching the 0.740
  release was refused, and the vendored `luau0-src` crate ships no `Analysis` sources to build it
  from. So strict analysis of the thirteen files is left to a run that has it. `build.rs` lints and
  compiles every one (`jackioh_luau::compile`), and each loads and runs in its card's tests. Every
  hook context is annotated (`J.EffectContext`). Every union-typed verb argument is a hoisted,
  annotated local (`J.TargetSpec` for `heal`, `J.AddToHandArgs`, `J.DelayArgs`), and string
  unions are annotated where a variable carries one (`J.SwitchAllPositionsSide`, `J.ScopeSide`,
  `J.RotationDirection`, `J.SetName`).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo jackioh catalog check`, `patches check` and `spec check` (805 rulings, 1710 proofs): clean.
- **`cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`**: section 6.

## 5. Decisions the plan did not settle

- **The parent was built in this worktree from the untouched base**, before any file changed, and
  copied out of the worktree before the next build, as part 15 did. Rejected: `git worktree add
  ../base` and a second full release build of the same commit.
- **Archivist passes its ids outright (section 1).** An empty table crosses into Rust as a record,
  not a list (`numbers::walk_table`), so a `cards` *function* answering an empty set is refused
  ("a cards function answers ids, not object"), and the hook panics. The radiant Archivist on an
  empty library would hit that, and its own test plays that case. Rejected: a `cards` function that never answers
  empty, which no honest translation gives, and editing `crates/luau`, which the plan forbids. The
  same refusal waits for any later card whose `for_each_card` set can be empty. It belongs in
  `api-gaps.md` beside section 2's gaps.
- **Recycling Initiative keeps storing `data.selfId`** though no Luau reader reads it back. Dropping
  it would change the delayed entry in state, and every state hash after it.
- **Silly Silas spells `radiant = "base"`.** The Rust gives both faces the same declaration and
  the same hook, and the radiant difference is `ctx.radiant`, which the rotation reads for itself.
  No test of the card compares the two faces' hooks.
- **Shared declaration tables** (Efficiency Dividend's `MODES` and `TARGETS`, Masochism Mask's
  `FLAGS`, Pocket Chaos's `SWAP_MODE`) are module-level, as part 9 shares `targets`. They are
  read-only data the loader deserializes, so L8 is unaffected.

## 6. Workspace tests

`cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`, then `cargo test
--no-fail-fast -p jackioh-tools -p jackioh-wasm` with the same features for the binaries after the
first failure:

- passing: the cards' unit tests 5,860 (this batch's 153 among them), the cross-card tests 752
  (`play_choices.rs`, `resolving_face.rs` and the four other files that play Pocket Chaos among
  them), the AI's 325 (§9.9's reply reasoning names #87 in `reply.rs` and `observe.rs`), the engine's
  unit tests 376, rules 2,637 and golden 19, the Luau host's 48, the server's 1,268 plus 10, the
  tools' unit tests 163 of 165, `acceptance.rs` 4 of 5 and `luau.rs` 4, and `apps/web/src/wire/`
  unchanged;
- failing: three tests of `jackioh-tools`, the three part 15's notes report failing the same way on
  its base. They are the surface tests `luau_core_002_bigot` and
  `luau_reports_print_as_markdown_and_json`, which survey `core-002` and `classic-090` and expect
  Bigot's Rust script that part 6 ported, and `acceptance.rs`'s
  `v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other`, which names
  `crates/cards/Cargo.toml`'s `jackioh-luau` dependency from part 4. None of them reads a file this
  part touches, and this part changes no manifest and no tool. They are the reconcile's.
