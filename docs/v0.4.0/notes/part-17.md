# v0.4.0 Part 17: cards to Luau, batch 9 (auras, static flags, costs, conditions and previews, Classic and Classic+)

Notes for issue #459 (v0.4.0 part 17 of 29), built by `docs/v0.4.0/PORTING.md`. Of the batch's 29
cards, 15 run from Luau now and 14 stay Rust (section 2). Every test in the 29 files is unchanged
except where a test called `script()` (section 3).

## 1. Cards ported

Each has a new `crates/cards/src/scripts/<set>/<module>.luau`. Its `.rs` lost `script()`, the
helpers and constants only `script()` used, and the `use` lines. It keeps its header comment,
`pub const ID` and `mod tests`. Where the Rust built one script for both faces (`base.clone()`),
the module says `radiant = "base"`, so the tests' `Arc::ptr_eq` checks still hold. Where it built
two, the module has two face tables.

| Card | Hooks | Notes |
|---|---|---|
| `classic-045` Nature Titan | `staticFlags.tribute`, `cry`, a trigger on `attackDeclared` | the Cry and the trigger share one local function, as the Rust shared `titan`; `radiant = "base"` |
| `classic-046` Divine Favor | `cry` (`draw_while` with `draw_more`), `preview` | the preview names the other seat itself (`ConditionContext` binds no `opponent_of`), as Classic #1 and Core #18 do |
| `classic-061` Plague Bringer Goliath | `staticFlags.tribute`, `cry` | `place_plague_tokens`, then `draw`, in the Rust order |
| `classic-080` BOOM! Big Max | `staticFlags.tribute` | keywords only, all printed |
| `classicplus-003` Second Amendment Snake | `endOfTurn`, `death`, `preview` | the hits read the hook's own snapshot (`ctx:self_().counters.plague`), so the Death keeps R78's last-known count |
| `classicplus-019-1` Top Loser | `staticFlags` (`attackedOnlyFromLane`, `neverDefense`) | |
| `classicplus-019-5` Bot Loser | a trigger on `destroyed`, `startOfTurn`, `endOfTurn`, `conditionMet`; radiant `neverBerserk` | Berserk is `berserk == true` on the instance, as `is_berserk` reads it; the radiant `conditionMet` answers `false`, as the Rust does |
| `classicplus-026` Tommy Tempo | `staticFlags.castOnDraw`, `cry` | `ctx:is_cast_on_draw(id)`, which reads `held_draws` by id as the Rust reads it by instance |
| `classicplus-033` Ivory Tower | `staticFlags.fusesCarried`, triggers on `cardResolved` (and radiant `cardPlayed`) | see section 4 |
| `classicplus-036-1` Bone Storm | `staticFlags.castOnDraw` (radiant also `echo = 1`), `cry` | one shared `cry` function, two face tables |
| `classicplus-039` Book Worm | `startOfTurn`, `death`, `preview` | the Death's count reads the snapshot, as `plague_on(ctx.self_)` did: clamped at 0 the same way |
| `classicplus-048` Jlockheed's Lobbyist | `staticFlags.neverDefense`, `death`; radiant `death` | |
| `classicplus-051` Jlockheed's J15 Fighter | `staticFlags` (`neverDefense`, `cantBeAttacked`) | |
| `classicplus-068` Organic Produce | `staticFlags.radiantPlaysTagged`, `cry` | radiant adds `costOverride = param("setCost")` |
| `classicplus-075-1` J-lease J-Jungle EX-plorer Pack | `staticFlags.castOnDraw`, `cry` | the radiant face adds `costOverride`, as the Rust `pack(true)` does |

Readers that answer an `Option` through serde (`self_`, `live_self`, `slot_of`, `carried_at`)
hand Luau mlua's `null`, which is truthy and not `nil`. The ports test such a value with
`type(x) == "table"`, as parts 11 to 16 do.

## 2. Cards that stay Rust

Each has its `.rs` exactly as at the base commit and no `.luau`. All three aura cards are among
them, so none of the batch's `luau bench` runs measured an aura (section 5).

- `classic-022` Mid Runner. A divergence the batch cannot fix (PORTING §5, reason 2), from the
  host gap parts 11 and 12 found: a `cards` function cannot answer an empty list (an empty table
  crosses as a record and `for_each_card` refuses it). R60 has the Rust draw the bounced cards
  lazily, as the list reaches the clause. In midlane the Cry tributes Mid Runner first, a §4.5 pass
  for one card, and that pass runs its Death hooks, a granted one included (ME-GRANT, MD-D13). So
  the hook cannot see whether the enemy will still have a permanent when the clause draws, and a
  lazy function that finds none panics. An earlier run's trial port had exactly that lazy
  function; it was dropped.
- `classic-036` Burn. Missing API: its `conditionMet` subtracts `effective_cost(self)` from the
  unspent mana, and `ConditionContext` binds no `effective_cost`.
- `classic-043` Plague Nuke. Missing API: the `preview` counts the Plague Counters on every Unit
  through `active_units_of`, which `ConditionContext` does not bind. The Cry also orders the
  Units from `state.active`, and no active-player reader is bound (#464).
- `classic-059` Plague Doctor. Missing API: the `preview` reads `permanents_on_field` and
  `plague_multiplier_of`, neither bound on `ConditionContext`.
- `classic-082` Sheeople. A divergence the batch cannot fix: its own test asserts
  `Arc::ptr_eq(base.death, radiant.death)` across two faces that differ in `tributeWorth`, and
  `load_card` builds one adapter per face hook (part 14, Chaos Machine). It also reads its
  `worth` param through `crate::card_def` (`api-gaps.md`).
- `classicplus-012-5` Anti-Waffle Shell. Missing API: its aura applies to the Units its controller
  has as the aura is asked (a set of ids), and the only aura verb bound is `defense_armor_aura`
  (`crates/luau/src/api/mod.rs`, `aura_entries`).
- `classicplus-012-6` Frozen Wastes. Missing API: the `preview` counts the doomed Units through
  `active_units_of` and `unit_view`, neither bound on `ConditionContext`.
- `classicplus-018` Gullible Treatler. Missing API: its `conditionMet` walks the backrow through
  `slots_of`, `card_at` and `card_type_of`, none bound on `ConditionContext`.
- `classicplus-046` Felinor Flagbearer. Missing API: its aura (your other Felinor Units, the tag
  read through `tags_of`, R923) has no aura verb; only `defense_armor_aura` is bound.
- `classicplus-046-1` Felinor Flagbearer Prime. Missing API: the same Felinor aura.
- `classicplus-047` Jogg's Box. A divergence the batch cannot fix: its own test asserts
  `Arc::ptr_eq(radiant.cry, base.cry)` across faces that differ in `echo`, which `load_card`'s one
  adapter per face hook cannot satisfy.
- `classicplus-050` Adaptive Growth. Missing API: its `conditionMet` compares the two sides'
  `active_units_of`, not bound on `ConditionContext`.
- `classicplus-064` Mulch Muncher. Missing API: its `cost` hook reads `param`,
  `playable_from_graveyard` and `played_this_game_with_tag`. The `cost` hook's `CostArgs` binds
  only `instance`, `hero_of` and `unspent_mana_of`. Its printed cost also comes from
  `crate::card_def` (`api-gaps.md`).
- `classicplus-t-ai-06` Datacenter Fire. Missing API: the `preview` calls `field_spells_doomed`
  through a `ConditionContext`, which binds no such reader. Its labels are also read off the
  catalog text through `crate::card_def` (`api-gaps.md`).

## 3. Test edits

None of the 206 tests in the 15 ported files changed an assertion (the base commit has 206
`#[test]`s in those files, and 206 pass). Where a test called `script()`, it now reads the
registry: `crate::scripts_of()[ID].clone()` (`super::ID` or `super::super::ID` where the test
already named it that way). Two test modules lost what they took from the parent's
`use jackioh_engine::prelude::*` through `use super::*`:

- Bot Loser's tests import `jackioh_engine::effects::go_berserk`, which one test builds;
- Jlockheed's Lobbyist's `use super::*;` now imports nothing (its tests name `super::ID`), so it
  is removed.

## 4. How Ivory Tower was made equal

The Rust reads `carried_at(state, slot_of(tower))`, which answers `None` for any slot outside the
backrow. The Luau reader `ctx:carried_at(player, lane)` always asks the backrow, so `rider_of`
first checks that the Tower's slot is in the backrow. It answers `nil` otherwise, as the Rust
does. `stacked_onto` crosses as a plain `Option<String>` (nil for `None`), so comparing it to
the event's `instanceId` matches the Rust `is_some_and(… == instance_id)`.

## 5. Results

The parent is `jackioh` built in release from the base commit `3381e9da9`, with every card still
Rust (`git worktree add --detach /tmp/base459 3381e9da9`, removed afterwards).

- `cargo test -p jackioh-cards --lib -- <the 15 modules>`: 206 passed, 0 failed.
- `cargo test -p jackioh-cards --lib`: 5860 passed. `cargo test -p jackioh-cards --test cards`
  (the cross-card tests, `condition_active.rs` and `preview.rs` among them): 752 passed.
- `cargo jackioh luau diff --parent-bin /tmp/base459/target/release/jackioh --cards classic-045,classic-046,classic-061,classic-080,classicplus-003,classicplus-019-1,classicplus-019-5,classicplus-026,classicplus-033,classicplus-036-1,classicplus-039,classicplus-048,classicplus-051,classicplus-068,classicplus-075-1`:
  200 games (seeds 1–200, 15,397 steps), `golden check: 200 of 200 games replayed identically`,
  `luau diff: 0 mismatches`. The four tokens were forced through the cards whose `refs` name
  them: Top Loser and Bot Loser through `classicplus-019` League of Losers, Bone Storm through
  `classicplus-036`, the Pack through `classicplus-075`.
- `cargo jackioh golden check`: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- `cargo jackioh fuzz --seeds 200`: 200 passed, 0 failed; 0 panics, 0 over the bound or stalled,
  0 replay mismatches, 0 invariant violations; 200 hero deaths.
- `cargo jackioh luau bench --parent-bin /tmp/base459/target/release/jackioh`, on linux x86_64
  with 4 threads (AMD EPYC 7763): "within every budget". It ran once, over all 15 ported cards.
  The plan asked for a run after each aura card; none of the three ported (section 2), so there
  was no aura to measure.

  | Number | Parent | This build | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` | 13,908.875 ms | 13,786.514 ms | ×0.99 | ≤ 2× |
  | Slowest decision | 250.567 ms | 250.660 ms | ×1.00 | ≤ 1.5× and < 1,500 ms |
  | Golden step | 2.001 ms | 2.005 ms | ×1.00 | ≤ 2× |

- `luau-analyze --mode=strict`: **not run**. This box has no Luau toolchain, and the sandbox
  refused the download of the 0.740 release. `build.rs` lints and compiles every new file, and
  each loads and runs in its card's tests. Every hook context is annotated (`J.EffectContext`,
  `J.ConditionContext`), and every union-typed or optional-field argument is a hoisted,
  annotated local where a field is set after the table is built or a union must narrow
  (`J.TargetSpec` for `heal` and `place_plague`, `J.FuseCardsArgs`, `J.AddRandomFromCatalogArgs`,
  `J.CatalogQueryArgs`, `J.StaticFlags`). The first run with the toolchain should check them.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo jackioh catalog check`, `patches check` and `spec check` (805 rulings, 1710 proofs):
  clean.
- `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts` (the binaries after
  the first failure rerun with `--no-fail-fast`, plus `--doc`): every binary passes but two in
  `jackioh-tools`, and `apps/web/src/wire/` is unchanged. Those two fail on three tests that are
  not this part's and that parts 10 to 18 report too. `--bin jackioh` (163 passed, 2 failed)
  fails `luau::surface::tests::luau_core_002_bigot` and `luau_reports_print_as_markdown_and_json`,
  because the Rust surface survey finds no script in Core #2 Bigot, which has been Luau since part
  9. `--test acceptance` (4 passed, 1 failed) fails
  `v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other`, because
  `crates/cards/Cargo.toml` depends on `jackioh-luau`. None of them reads a file this part changed.

## 6. Decisions the plan did not settle

- **Mid Runner stays Rust rather than half-ported.** Rejected: reading the ids in the hook and
  passing them outright (`cards = ids`) when it is not in midlane, and keeping the lazy function
  only in midlane. Outside midlane that is equal, because nothing runs between the hook and the
  clause. In midlane the empty answer still panics whenever the tribute's own Death empties the
  enemy board.
- **Snapshot counters, not `ctx:plague_on(id)`.** Second Amendment Snake and Book Worm read
  `counters.plague` off `ctx:self_()`. `EffectContext.plague_on` resolves the id to the live
  instance, which in a Death hook is the graveyard card after R78's reset.
- **Static-flag-only cards keep `local J = require("@jackioh")`** (BOOM! Big Max, Top Loser, J15
  Fighter), as PORTING §2 opens every file and as Core #66 The Rock does. Rejected: dropping it
  as part 11's empty cards did. That would leave these files unlike every other static-flag port.
- **`api-gaps.md` is not edited here**, so the parallel batches don't conflict on it. The gaps in
  section 7 are for the reconcile to fold in.

## 7. Host findings for the reconcile (not fixed here: `crates/luau` is not this part's)

- **`ConditionContext` binds too few readers** for this batch's previews and glows: no
  `active_units_of` (Plague Nuke, Frozen Wastes, Adaptive Growth), `unit_view` (Frozen Wastes),
  `permanents_on_field` or `plague_multiplier_of` (Plague Doctor), `effective_cost` (Burn),
  `slots_of`, `card_at` or `card_type_of` (Gullible Treatler), and no `field_spells_doomed`
  (Datacenter Fire). Each is an `EffectContext` reader already, so the fix is to register the
  same methods on `ConditionContext`.
- **`CostArgs` binds no `param`, `playable_from_graveyard` or `played_this_game_with_tag`**
  (Mulch Muncher).
- **Auras have one verb, `defense_armor_aura`.** An aura over a set of ids (Anti-Waffle Shell) or
  over a tag (Felinor Flagbearer and its Prime) needs a data-shaped `applies`.
- **Two faces cannot share one hook `Arc`** unless the whole face is shared (`radiant = "base"`).
  Sheeople and Jogg's Box assert it, as Chaos Machine did in part 14.
- The empty-list gap of parts 11 and 12 also keeps Mid Runner.
