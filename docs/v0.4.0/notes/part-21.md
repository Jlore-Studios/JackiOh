# v0.4.0 Part 21: cards to Luau, batch 13 (Activate abilities, replacements and target checks)

Notes for issue #463 (v0.4.0 part 21 of 29, a fullsend part of #442): the 22 Classic and Classic+
cards of batch 13 and the 5 Core cards earlier batches held back (#442, L27), ported by
`docs/v0.4.0/PORTING.md` onto part 28's fixed API (#464, `docs/v0.4.0/api-gaps.md` §"Fixed in part
28"). 24 of the 27 now run from a `.luau` file. 3 stay in Rust because the API cannot say them
(PORTING.md §5, reason 1). Base commit: `5e292a8fb` (`bot/issue-442` with #464 merged; every card of
this batch still Rust there).

## 1. Ported to Luau

Each `.luau` sits beside its `.rs`. The `.rs` keeps its `//!` header, `pub const ID`, the comment
above its tests and its `#[cfg(test)] mod tests`. Its `script()`, the helpers and constants only
`script()` used, and its `use` lines are gone. Every effect keeps the Rust order. Every ability,
replacement and trigger id, label, mode option, `resume` key and `step` is the Rust one, because
`legal_actions`, a paused state and a replayed log name them. Every declared number is still a
`ctx:param` read (R386).

| Card | Id | What it declares |
|---|---|---|
| C #7 InfiniScepter | `classic-007` | a hand `targets` pick checked by `targetChecks.spellWithinLimit` (`effective_cost` ≤ `param("costLimit")`); `cry`: `remember` then `exile`; one Activate whose `canActivate` reads `self_().memory.scepter` and whose `run` casts `ctx:recalled("scepter")` (`cast_new`); `radiant = "base"` |
| C #15 Nose Hunter | `classic-015` | Activate ♾️ with `cost = { discardRandom = 1 }`; radiant adds `exile_random_from_hand` |
| C #20 The Power to Punish | `classic-020` | one Activate with `modes` and two `forModes` targets (radiant: one), `destroy_at_next_turn_start` with the red mark |
| C #21 Turtinator | `classic-021` | Activate ♾️ with `cost = { tribute = 1, tributeExcludesSelf = true }`; the hit reads `ctx:subsystems_activation_paid().tributed[1].attack` (part 6's reader); `radiant = "base"` |
| C #23 Devil's Pact | `classic-023` | `cry` (`discard_hand` / `discard_random`); one Activate adding `replacePlays` until `ctx:turn().number` ends (gap 4) |
| C #33 Joro | `classic-033` | one replacement, data only: `targeted`, `where = "hand"`, `by = "spell"`, `instead = { interpose = true }`; `radiant = "base"` |
| C #48 Hired Shrimp | `classic-048` | `targets` (radiant names `check = "longer"`), `targetChecks.longer` on both faces (`def_of(..).loc`, `unreadable_by`), `cry` |
| C #50 Voidwalker | `classic-050` | `cry`: `exile_matching`; a `toGraveyard` replacement into exile; radiant's `when` reads `ctx:event().owner` |
| C #52 Final Gambit | `classic-052` | a `lethalHit` replacement redirecting to the enemy hero, `then = "afterRedirect"`; `resume.afterRedirect` reads `ctx:data().replaced.redirectedTo` (gap 13) |
| C #54 Rewind | `classic-054` | `targets` with `check = "hasCry"` (`has_triggerable_cry`), `cry`: `param("repeats")` × `trigger_cry({})` |
| C #60 Pile On | `classic-060` | `cry`: `recruit_all({})`; base adds a `self` `toGraveyard` replacement to the bottom of the deck; one `cry` function on both faces, so one adapter (gap 14, the Rust's `base.cry.clone()`) |
| C #78 Mutate Spell | `classic-078` | Activate ♾️, a `plague = true` target, `canActivate` over `permanents_on_field` × `plague_on`, `run`: `consume_plague`, then exile / `fuse_onto_your_card`, draw or `forced_attack_random` |
| C #81 The Power to Thrive | `classic-081` | one Activate with three modes; `radiant = "base"` |
| C #84 Lockdown | `classic-084` | a `cardPlayed` trigger (`lock_played_zone({ event })`, never its own arrival, radiant only the opponent's plays); Activate with `cost = { tributeSelf = true }` and an empty `run` |
| C+ #22 Blood Moon | `classicplus-022` | a `healed` replacement (`damage = "pierce"`, base `lasting = "thisTurn"`); radiant `staticFlags.healToDamage` |
| C+ #31 Fusion Lab | `classicplus-031` | Activate with a hand pick `check = "fusable"` (`targetChecks`: not `unit_has(.., "Immutable")`), `run`: `fuse_random_into` |
| C+ #41 KY's Constant | `classicplus-041` | hand `targets` with `excludeSelf` and `check = "number"` (`numbers_on`); base `set_number` random; radiant `discover_number` and `resume.picked` (`chosen_tuning_number`) |
| C+ #60 Doctors Orders | `classicplus-060` | one Activate: `param("apples")` × `add_to_hand` of C+ #59, of the face's kind |
| C+ #76.1 Brother Ping | `classicplus-076-1` | one Activate, a unit-or-hero target; `uses` 1 and 2 |
| Core #35 Lunar Eclipse | `core-035` | `targets`, `cry`: `damage` then a `costDiscount` expiring `{ until = "thisTurn", turn = ctx:turn().number }` (gap 4); `radiant = "base"` |
| Core #42 Eugenics | `core-042` | `cry`: `exile_random_from_library`, then `radiant_chance` with `chancePercent` 30 / 40 (gap 9) and `lucky` = printed + `given_lucky_on(live_self)` (gap 5) |
| Core #77 Professor Curvature | `core-077` | `cry`: a `costDiscount` with `minCurrentCost`, expiring `nextTurnOf` from `ctx:turn().number` (gap 4); `radiant = "base"` |
| Core #83 Transmogulate | `core-083` | `cry`: one `transform` per board, hand, library (`numbering_order`, gap 1), graveyard and exile card, each a `ctx:rng_pick` over `J.pool` |
| Core #23 Reoccurring Dream | `core-023` | `cry`: `1 + max(lucky, 0)` `ctx:rng_chance(3 or 4, 10)` draws, no short circuit, then `set_radiant_random`; `endOfTurn`: the R68 `bounce` |

Five translations are not one-to-one with the Rust. Each gives the same answers:

- **InfiniScepter's cast reads its Spell in the hook, not as the effect applies.** The Rust hands
  `cast_new` a `CastNewDef::Read` closure, which has no Luau form (serde skips it). The `run` hook's
  list is that one effect, so nothing runs between the hook and the read, and `ctx:recalled` is the
  Rust's `held_spell(ctx.live_self(), &ctx.data)` (the running ingredient's key on a fused card,
  R102). With no Spell held the hook answers `{}`, where the Rust's read answers `None` and the
  effect casts nothing.
- **Transmogulate's off-field pool is queried once.** The Rust calls `pool` for every off-field
  card; the query is the same each time and reads no state. The board's pools stay per card, by
  that card's type.
- **Transmogulate walks the board through `ctx:permanents_on_field(me)`**, keeping the cards on the
  controller's side (`zone.player == me`): that reader walks the controller's units, then their
  backrow, lane by lane (the Rust's `slots_of` × `card_at`), then the other side. `ctx:card_at` is
  Units-only in Luau.
- **Eugenics and Reoccurring Dream state their chances as integers.** `chancePercent = 30` is
  converted `f64::from(30) / 100.0` (`api/radiant.rs`), and `ctx:rng_chance(3, 10)` is `3.0 / 10.0`.
  Both divisions are correctly rounded, so each is the double the Rust literal `0.3` (or `0.4`)
  names, and the rng draws are the same.
- **KY's Constant reads `chosen_tuning_number()` by its Rust field names.** `ChosenTuningNumber` has
  no `rename_all`, so it crosses as `{ instance_id, which }`, not camelCase. The first draft read
  `pick.instanceId`, which is `nil`: `set_number` then named no card, and three radiant tests failed
  until the read was fixed.

## 2. Kept in Rust

Each keeps its `.rs` exactly as the base commit has it and has no `.luau`.

| Card | Id | Why |
|---|---|---|
| C #14 Shadowstep | `classic-014` | its base trigger reads the whole §4.5 pass it stands in from the action's full event list (`ctx.events[..]`: the unbroken run of `destroyed` events around the one it answers). `ctx:events()` answers only the script's own window, `events[events_from..]` (R136, gap 6), and a trigger's window opens when its run begins (`triggers.rs`, `make_context`), after the pass, so the Luau would see none of it |
| C #32 Felinor Feelings | `classic-032` | its radiant `choose_target_where` admits only enemy permanents in a lane where its controller has a (1) Cost Unit, through a `where_` closure. The Luau verb deserialises `ChooseTargetWhereArgs`, whose `where_` serde skips, so it would offer every enemy permanent. The lanes have to be read after the Cry's own summon, so a precomputed set will not do either |
| C #42 Transmutable Toxins | `classic-042` | its aura answers one entry per Plague-carrying unit, each applying to that unit's id alone with that unit's count. The generic aura (gap 8) cannot name one instance: `AuraFilter` has side, rows and def ids, and a named `aura_filter` gets only the candidate, with no data to tell which entry it is |

These belong in `docs/v0.4.0/api-gaps.md`: a reader for the action's events before the script's
window (or the pass a trigger answers) for Shadowstep, a named `where` predicate for
`choose_target_where` (as `aura_filter` is for the aura) for Felinor Feelings, and an instance
filter (or data handed to `aura_filter`) for Transmutable Toxins. Also: the typed stub
`J.pool(name: string): {string}` in `crates/cards/luau/jackioh.luau` does not match the binding,
which takes the excluded id and the query and answers definitions (`api/mod.rs`, `pool_catalog`).
Transmogulate calls it through `(J.pool :: any)`. This part does not edit those files: the plan
names only the card files and these notes, and the batches build side by side. #442's reconcile
folds them in.

## 3. The tests

No test asserts anything new or anything less. The three Rust cards' files are untouched. In the 24
ported files (442 tests), these lines changed, all because `script()` and the module's `use` lines
are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()`, the registry `build.rs` generates,
  in 19 files: 17 read `[ID]`, KY's Constant `[super::ID]` and Brother Ping `[super::super::ID]`,
  where their tests name it. Lockdown's, Doctors Orders', Lunar Eclipse's, Professor Curvature's and
  Transmogulate's tests never called `script()`.
- `use super::{script, ID};`, `use super::{ID, script};` and `use super::script;` are
  `use super::ID;`.
- Three test modules used names the removed `use jackioh_engine::prelude::*;` brought in through
  `use super::*;`, so they import them themselves: `use std::sync::Arc;` in InfiniScepter's and
  Turtinator's (their `Arc::ptr_eq` checks; part 28 did the same for Rapid Draw), and
  `use jackioh_engine::prelude::*;` in Blood Moon's (`set_health`).
- Transmogulate's `use super::*;` imported nothing its tests use any more, so it is removed, as
  parts 14 and 18 removed such lines.

## 4. Proofs

All run on this build, in this worktree, on linux x86_64, 4 threads, Intel Xeon 6973P-C. The
parent `jackioh` was built from the base commit in this worktree before any file changed and copied
out of it.

- **Card tests.** `cargo test -p jackioh-cards --lib --` with the 24 ported modules set-qualified
  (`classic::c007_infiniscepter` … `core::c023_reoccurring_dream`): 442 passed, 0 failed. The whole
  crate, `cargo test -p jackioh-cards`: unit tests 5,860 passed, the cross-card tests (`tests/cards.rs`,
  `fused_hooks.rs` among them) 752 passed.
- **`luau diff`**, parent built from the base commit. The 24 cards do not fit one 20-card deck
  ("24 forced cards do not fit a deck of 20"), so three runs of 200 games each, seeds 1–200:
  - `classic-007, -015, -020, -021, -023, -033, -048, -050, -052, -054, -060, -078`: 20,607 steps,
    **0 mismatches**;
  - `classic-081, -084, classicplus-022, -031, -041, -060, -076-1, core-035, -042, -077, -083,
    -023`: 20,050 steps, **0 mismatches** (Brother Ping, a token, goes in through Brother Lar,
    `classicplus-076`, R279);
  - the three Rust cards with nine ported ones they meet in play (`classic-014, -032, -042, -078,
    -033, -054, -084, classicplus-031, classic-021, -007, -052, core-083`): 18,776 steps,
    **0 mismatches**.
- **What those games ran.** The first two runs re-recorded with `golden record --seeds 1..200` and
  the same forced lists, then folded by a throwaway counter outside the repository: activations
  (`activated`, base / radiant face of the card activated), resolutions (`cardResolved`, base /
  radiant) and fired traps.

  | Card | Activations | Resolved | Other |
  |---|---|---|---|
  | InfiniScepter | 454 / 0 | 275 / 5 | |
  | Nose Hunter | 559 / 2 | 300 / 3 | |
  | The Power to Punish | 1,552 / 8 | 241 / 1 | |
  | Turtinator | 440 / 21 | 224 / 8 | |
  | Devil's Pact | 1,600 / 25 | 266 / 7 | |
  | Joro | | 306 / 2 | |
  | Hired Shrimp | | 287 / 4 | |
  | Voidwalker | | 262 / 1 | |
  | Final Gambit | | 229 / 0 | 92 fired |
  | Rewind | | 345 / 7 | |
  | Pile On | | 24 / 1 | |
  | Mutate Spell | 119 / 1 | 267 / 4 | |
  | The Power to Thrive | 586 / 49 | 166 / 15 | |
  | Lockdown | 166 / 12 | 185 / 15 | |
  | Blood Moon | | 191 / 7 | 132 fired |
  | Fusion Lab | 661 / 30 | 182 / 11 | |
  | KY's Constant | | 230 / 9 | |
  | Doctors Orders | 670 / 21 | 203 / 9 | |
  | Brother Ping | 265 / 18 | | |
  | Lunar Eclipse | | 244 / 9 | |
  | Eugenics | | 141 / 2 | |
  | Professor Curvature | | 165 / 8 | |
  | Transmogulate | | 205 / 18 | |
  | Reoccurring Dream | | 408 / 40 | 412 end-of-turn returns (`bounced`) |

- **`golden check`**: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- **`fuzz --seeds 200`**: 200 passed, 0 failed; 0 panics, 0 over the bound or stalled, 0 replay
  mismatches, 0 invariant violations; 200 hero deaths.
- **`luau bench --parent-bin`**: within every budget.

  | Number | Parent (base) | This build | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall time | 11,372.104 ms | 12,814.144 ms | ×1.13 | ≤ 2× |
  | Slowest `gate` decision | 120.612 ms | 106.055 ms | ×0.88 | ≤ 1.5× and < 1,500 ms |
  | Golden step | 1.699 ms | 1.866 ms | ×1.10 | ≤ 2× |

- **`luau-analyze --mode=strict`: not run.** This box has no Luau toolchain and fetching the 0.740
  release was refused. `build.rs` lints and compiles every new file (`jackioh_luau::compile`), and
  each loads and runs in its card's tests. Every file starts with `--!strict`; effect, condition and
  replacement contexts are annotated (`J.EffectContext`, `J.ConditionContext`,
  `J.ReplacementContext`); union-typed verb arguments are hoisted into annotated locals
  (`J.TargetSpec` for `heal` and Mutate Spell's target, `J.DestroyAtNextTurnStartArgs`,
  `J.RadiantChanceLuauArgs`). Target checks take an unannotated `args`: `jackioh.luau` exports no
  type for `TargetCheckArgs`. Activations declare `uses = "unlimited"`, which the loader reads
  (`script.rs`) and the `ActivationUses = number | nil` alias does not name; the face tables are not
  annotated, so strict checks no literal against it. The first run with the toolchain should check
  the 24 files, and the `J.pool` stub (section 2).
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo jackioh catalog check`,
  `patches check` and `spec check` (805 rulings, 1710 proofs): clean.
- **`cargo fmt --check`** fails on one file this part does not touch,
  `crates/luau/tests/api.rs:1101` (an `assert!` rustfmt would put on one line), exactly as at the
  base commit. The plan forbids `crates/luau`, so it is left for the reconcile. Every file this part
  changed is formatted.
- **`cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`**: section 6.

## 5. Decisions the plan did not settle

- **The parent was built in this worktree from the untouched base**, before any file changed, and
  copied out of the worktree, as parts 15 and 18 did. Rejected: `git worktree add ../base` and a
  second full release build of the same commit.
- **Three `luau diff` runs instead of one.** `luau diff` puts every forced card in a 20-card deck, so
  24 cannot go in one run. Rejected: dropping cards from the proof.
- **One function under both faces' hooks where the Rust built two closures** (Devil's Pact's `cry`,
  Reoccurring Dream's `endOfTurn`): the faces then share one adapter (gap 14). The engine never
  compares hooks by pointer, and no test of either card does. Efficiency Dividend (part 18) does the
  same. Pile On's `cry` shares one function because its Rust does (`base.cry.clone()`) and its test
  asserts `Arc::ptr_eq`.
- **Shared declaration tables** (Nose Hunter's `COST`, Power to Punish's targets and mark, Final
  Gambit's `REPLACEMENTS`, Lockdown's `TRIBUTE`, Hired Shrimp's `TARGET_CHECKS`, KY's Constant's
  `TARGETS`, Brother Ping's `TARGETS`) are module-level, as parts 9 and 18 share theirs. They are
  read-only data the loader reads once per face, so L8 is unaffected.
- **Mutate Spell's `canActivate` reads Plague through `ctx:plague_on(card.id)`**, the engine's read
  helper, not `card.counters.plague`: the same number, read the way the Rust reads it.
- **Shadowstep, Felinor Feelings and Transmutable Toxins were not ported partly.** A card is one
  script source (`build.rs`), so a face that cannot be said keeps the whole card in Rust.

## 6. Workspace tests

`cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts --no-fail-fast`:
11,480 passed, 3 failed.

- passing: the cards' unit tests 5,860 (this batch's 442 among them), the cross-card tests 752, the
  AI's 325, the engine's unit tests 376, rules 2,637, golden 19 and `export_config` 2, the Luau
  host's 1 + 31 (`api.rs`) + 1 (`export_api.rs`) + 27 (`host.rs`), the server's 7, 3 and 1,268, the
  tools' unit tests 163 of 165, `acceptance.rs` 4 of 5 and `luau.rs` 4; and `apps/web/src/wire/`
  unchanged;
- failing: the three tests of `jackioh-tools` that parts 15 and 18 report failing the same way on
  their bases. `luau_core_002_bigot` and `luau_reports_print_as_markdown_and_json` survey `core-002`
  and `classic-090` and expect Bigot's Rust script, which part 6 ported; `acceptance.rs`'s
  `v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other` names
  `crates/cards/Cargo.toml`'s `jackioh-luau` dependency from part 4. None of them reads a file this
  part touches, and this part changes no manifest and no tool. They are the reconcile's.
