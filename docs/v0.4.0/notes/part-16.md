# v0.4.0 Part 16: cards to Luau, batch 8 (auras, static flags, costs, conditions, previews)

Notes for issue #458 (v0.4.0 part 16 of 29, a fullsend part of #442): the 29 Core/Classic
cards the engine reads constantly rather than once, ported by `docs/v0.4.0/PORTING.md`.
Eighteen now run from a `.luau` file; eleven stay in Rust for a binding the API does not
have (PORTING.md §5, reason 1). Base commit: `c0f66509a` (`bot/issue-442` with parts 6, 7
and the earlier batches, every card of this batch still Rust).

## 1. Ported to Luau

Each `.luau` sits beside its `.rs`, which keeps its header, `pub const ID`, its comments and
its `#[cfg(test)] mod tests`, with `script()`, its helpers and its now-unused `use` lines
removed. Every effect is in the Rust order; trigger ids are the Rust ones. A card whose faces
act alike spells `radiant = "base"`; one whose faces differ only in a declared number reads it
through `ctx:param`, so a Degrade or an Upgrade moves the Luau card exactly as the Rust one.

| Card | Id | What it declares |
|---|---|---|
| Core #18 Bread and Butter | `core-018` | a `turnEnded` trigger (`when` plus the empty-list guard, as in Rust): `summon` the Bread Token with `statsOverride` and `armorOverride` off the event's unspent mana times `param("multiplier")`; `preview` and `conditionMet` through `your_turn` and `unspent_mana_of`; `radiant = "base"` |
| Core #27 Blood Ridden Glowy Jelly Bean | `core-027` | `staticFlags = { castOnDraw = true }`; `cry`: `set_radiant_random` over the hand of `param("cards")`, then `lose_health` to self of `param("loss")` |
| Core #38 Quickstriker | `core-038` | `staticFlags = { quickstriker = true }`; `preview` X = `cards_played_this_turn`; `radiant = "base"` (the Radiant 2X is the engine's multiple, as is #84's Armor) |
| Core #40 Echoes of the Forgotten | `core-040` | per-face `startOfTurn`: `damage` to the enemy hero of `param`-multiple times the controller's exile, then `exile_bottom_of_library` (the count before the new card, as in Rust); `preview` with the face's formula |
| Core #53 Reno | `core-053` | `cry`: `heal` the self hero `upTo param("health")` when strictly below it; `conditionMet` hand-only with the same strict comparison |
| Core #55 Lava Golem | `core-055` | `staticFlags` `tribute = 3`, `tributeEnemies` on both faces, plus `enemyTributeHandsOver` on base (R360) |
| Core #64 Gifted Program | `core-064` | `staticFlags = { giftedProgram = 1 }`, 2 on radiant (the engine reads the `giftLimit` number through `declared_or`, as before) |
| Core #66 The Rock | `core-066` | `staticFlags = { tribute = 1 }`; `radiant = "base"` (the radiant keyword is catalog data) |
| Core #68 Twisted Sorcerer | `core-068` | `targets` (any side, unit or hero); `cry`: `damage` to chosen of `param("damage")`, or `param("lowDamage")` when the controller's hero is strictly below `param("threshold")`; `conditionMet` with the same reading |
| Core #69 Call to Arms | `core-069` | `cry`: three `recruit`s in order, Units costing at most `param("costLimit")`, for self |
| Core #71 Intern Stimmy | `core-071` | a `turnEnded` trigger (`when` plus the `run` guard, so a miss leaves the trap armed) and `conditionMet`: the controller's library strictly larger than the opponent's; `recruit` a Unit costing at most `param("costLimit")` |
| Core #73 Anti-oneshot Armor | `core-073` | `staticFlags = { antiOneshot = true }`; `cry`: `draw param("draw")` |
| Core #75 Infinite Reserves | `core-075` | `staticFlags = { infiniteReserves = true }`; the radiant face adds `cry`: `draw param("draw")` |
| Core #79 Twinspell | `core-079` | `staticFlags = { echoGrant = 1 }`, 2 on radiant (read through `declared_or`, as before) |
| Core #84 Going Long | `core-084` | `staticFlags = { quickdraw = true, heroArmor = true }`; `radiant = "base"` (values live in `config.HERO_ARMOR`, read through `declared_or`) |
| Core #91 Fed Fauci | `core-091` | a `damage` trigger (`when` plus the `run` guard): +1 Plague Counter on a hit whose target is this card; `startOfTurn`: `gain_mana` of own counters times `param("mana")`, nothing when zero; `preview` with the face's formula; `radiant = "base"` |
| Classic #1 Curse of the Forgotten Classic | `classic-001` | `cry`: one `damage` hit of `param("damage")` times their exile, then `draw param("draw")`; radiant then `recruit`s their newest permanent from exile and, if it is a Unit, makes one `forced_attacks` on the enemy hero named by def id with `summonedThisScript`; shared `preview` |
| Classic #19 Lizard's Breath | `classic-019` | `targets` (any side, unit or hero); `cry`: `damage` of `param("damage")` plus `param("extraDamage")` when Exile counts, then `draw` when Deck counts, then `gain_mana` when Graveyard counts; `preview` names the counting piles in rank order with their sizes; piles ranked largest-first over the text order with `J.sort_by` (stable, so ties keep the text order as the Rust `sort_by` does) |

`ConditionContext` binds no `opponent_of`, so the three scripts that need the other seat in a
condition or preview (Bread and Butter, Intern Stimmy, Curse Classic) carry a local `p1`↔`p2`
helper; their effect contexts use the bound `ctx:opponent_of`. Lizard's Breath narrows trigger
events on `event.type` before reading fields; union-typed verb arguments are hoisted into
annotated locals (`J.TargetSpec`, `J.RecruitFilter`, `J.ForcedTarget`).

## 2. Kept in Rust

Each keeps its `.rs` exactly as the base commit has it and has no `.luau`. All eleven wait on
a name the API does not bind (reason 1); none is slow and none diverged.

| Card | Id | Why |
|---|---|---|
| Core #10 Rapid Replenish | `core-010` | `condition_met` (R195) reads `played_earlier(state, controller, live-self)`; `ConditionContext` binds no `played_earlier`, and the bound one is `EffectContext`-only and takes a def-id string, not the live self |
| Core #14 Jlockeed's Weapons | `core-014` | its `aura_hook` carries a custom `applies` (+4 attack, Rush, First Strike to your units); `api::aura_entries` binds only `defense_armor_aura` |
| Core #31 KY's Math Equation | `core-031` | the preview label comes from `crate::card_def` (already in `api-gaps.md`) and its value from `times_played_of` plus `fib` on the `ConditionContext`, neither bound there |
| Core #46 Suppressive Aura | `core-046` | two `aura_hook`s with custom `applies` (all units on base, enemy units on radiant); no generic aura verb |
| Core #58 Rush Token Farm | `core-058` | `rush_token_aura` is a custom `applies` over §10.4's computed layer; no generic aura verb |
| Core #60 Bear Honeypot | `core-060` | `condition_met` reads `open_zones` on the `ConditionContext`, unbound there (the effect context binds it, but one source per card keeps the whole card Rust) |
| Core #65.1 Spikey Pillow | `core-065-1` | `attack_drain_aura` with a per-face `applies` (your units vs your non-Pillow units); no generic aura verb. A token, summoned by #65 |
| Core #70 Spiteful Stab | `core-070` | the preview label is `fill_params` of `crate::card_def` with `params_view`, and the scaling reads `HERO_HEALTH`; `card_def` is already in `api-gaps.md`, and neither `params_view` nor the constant has a Luau form on the `ConditionContext` |
| Core #85 Unlicensed Experimentation | `core-085` | `condition_met` reads `fusable_permanents_of` on the `ConditionContext`, unbound there |
| Core #100 Ceaseless Void | `core-100` | the `cost` hook reads `state.counters` and `param("discount")`, but `CostArgs` binds only `instance`, `hero_of` and `unspent_mana_of`; its `condition_met` needs `play_cost`, unbound on the `ConditionContext` |
| T-sheep Sheep Token | `core-t-sheep` | the one static flag's values are the engine's named constants `SHEEP_TRIBUTE_VALUE` / `RADIANT_SHEEP_TRIBUTE_VALUE` (rule 9), unbound in Luau; a literal would duplicate them. Its test also asserts the Rust `Script` struct directly through `script()` |

The aura-hook cards are #14, #46, #58 and #65.1: all four stay Rust, so no Luau aura runs
inside any stat read and there is nothing bench-card-by-card to report. The new gaps these
eleven wait on (a generic aura verb; on the `ConditionContext`: `played_earlier`,
`times_played_of`, `open_zones`, `fusable_permanents_of`, `params_view`, `play_cost`; on
`CostArgs`: counters and `param`; the Sheep tribute constants) belong in
`docs/v0.4.0/api-gaps.md`. This part does not edit that file (the plan names only the card
files and these notes, and nineteen batches build side by side); #442's reconcile folds them
in.

## 3. The tests

No test asserts anything new or anything less: removed lines with `assert` are zero, added
lines with `assert` are zero, and the eleven Rust cards' files are untouched. In the eighteen
ported files (256 tests) two kinds of line changed, both because `script()` and the module's
`use` lines are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()` (three sites; six more in
  nested modules spell the id `super::super::ID`); `use super::{ID, script};` is
  `use super::ID;` (Quickstriker), and `use super::script;` (Gifted Program) and
  `use super::*;` (Twinspell, Going Long, Fed Fauci) are dropped, their tests naming their
  own constants. `scripts_of()` is the registry `build.rs` generates, so each test reads
  the Luau card exactly as the engine registers it.

## 4. Proofs

All run on this build, in this worktree, on linux x86_64, 2 threads, Intel Xeon Platinum 8488C.
Parent oracle: `../base-458/target/release/jackioh`, release-built from the base commit
`c0f66509a` in a scratch worktree (since removed); the two binaries' hashes differ, so the
oracle is genuinely the Rust build.

- **Card tests.** `cargo test -p jackioh-cards --lib -- core::c018_bread_and_butter
  core::c027_blood_ridden_glowy_jelly_bean core::c038_quickstriker
  core::c040_echoes_of_the_forgotten core::c053_reno core::c055_lava_golem
  core::c064_gifted_program core::c066_the_rock core::c068_twisted_sorcerer
  core::c069_call_to_arms core::c071_intern_stimmy core::c073_anti_oneshot_armor
  core::c075_infinite_reserves core::c079_twinspell core::c084_going_long
  core::c091_fed_fauci classic::c001_curse_of_the_forgotten_classic
  classic::c019_lizards_breath`: 256 passed, 0 failed. Full `jackioh-cards` lib suite:
  5,860 passed, 0 failed; `--test cards` (cross, incl. `condition_active.rs` and
  `preview.rs`): 752 passed, 0 failed.
- **`luau diff`**, three runs, views compared (R280's previews and R195's yellow glow live in
  the views):
  - the eighteen ported cards forced into both decks: 200 games, seeds 1–200, 21,338 steps,
    **0 mismatches**;
  - nine stay-Rust cards (Rapid Replenish, Jlockeed's Weapons, KY's Math, Suppressive Aura,
    Rush Token Farm, Bear Honeypot, Spiteful Stab, Unlicensed Experimentation, Ceaseless
    Void) meeting seven ported cards in play: 200 games, 15,291 steps, **0 mismatches**;
  - the two stay-Rust tokens brought by their generators (#65 Masochism Mask for the Spikey
    Pillow, #41 Sheepish for the Sheep Token) meeting five ported cards: 200 games,
    18,423 steps, **0 mismatches**.
- **`golden check`**: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- **`fuzz --seeds 200`**: 200 passed, 0 failed; 0 panics, 0 over the bound, 0 replay
  mismatches, 0 invariant violations.
- **`luau bench --parent-bin`**: within every budget.

| Number | Parent (base) | This build | Ratio | Budget |
|---|---|---|---|---|
| `fuzz --seeds 200` wall time | 23,917.5 ms | 23,985.9 ms | ×1.00 | ≤ 2× |
| Slowest `gate` decision | 235.0 ms | 227.9 ms | ×0.97 | ≤ 1.5× and < 1,500 ms |
| Golden step | 1.823 ms | 1.863 ms | ×1.02 | ≤ 2× |

- **`luau-analyze --mode=strict`: not run.** There is no Luau toolchain on this box (same as
  part 15). `build.rs` lints and compiles every new file (`jackioh_luau::compile`), and each
  loads and runs in its card's tests. Every hook context is annotated (`J.EffectContext`,
  `J.ConditionContext`), events are narrowed on `event.type` before a field is read, and the
  files use only `ipairs`, `J.sort_by`, and the allowed `math` members (`max`, `min`).
- `cargo fmt --check`, `cargo clippy -p jackioh-cards --all-targets -- -D warnings`,
  `cargo jackioh catalog check`, `patches check` and `spec check` (805 rulings, 1710 proofs):
  clean.

## 5. Decisions the plan did not settle

- **The parent was built in a scratch worktree at the base commit**, per PORTING.md step 1
  (`git worktree add ../base-458 c0f66509a`, release `jackioh-tools`, worktree removed
  after). Rejected: building in this worktree, whose files already held the ports.
- **The Spikey Pillow and the Sheep Token reach `luau diff` through their generators**
  (#65, #41). Rejected: leaving the two tokens out of the diff, since their auras and worth
  then never meet the ported cards in play.
- **New API gaps are written here, not in `api-gaps.md`** (section 2), as part 15 did: the
  plan names only the card files and these notes, and nineteen batches build side by side.
- **`script()` in a test reads `crate::scripts_of()`**, the crate's generated registry, as
  part 15 did. Rejected: `jackioh_engine::scripts::registered_entry`, which answers only
  after `register_all()` and would make the tests depend on registration order.
