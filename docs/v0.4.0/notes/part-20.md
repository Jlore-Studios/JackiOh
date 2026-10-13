# v0.4.0 Part 20: cards to Luau, batch 12 (the rare hooks)

Notes for issue #462 (v0.4.0 part 20 of 29), built by `docs/v0.4.0/PORTING.md` on the base
commit `5e292a8fb` (part 28's build on `bot/issue-442`). Of the batch's 29 cards (26, plus three
held back by batches 6 and 7), 26 run from Luau now and 3 stay Rust (section 2). No test
assertion changed (section 3).

## 1. Cards ported

Each has a new `crates/cards/src/scripts/<set>/<module>.luau`. Its `.rs` lost `script()`, the
helpers and constants only `script()` used, and the `use` lines. It keeps its header comment,
`pub const ID` and `mod tests`. Where the Rust built one script for both faces (`base.clone()`),
the module says `radiant = "base"`, so the tests' `Arc::ptr_eq` checks hold. Where it shared one
direct hook between two different faces (Living Bomb's `startOfOpponentTurn`, Prem Panther's
`afterAttack`), the module names one local function in both face tables (#464 gap 14). Where it
built two (Classic Golem's `after_it_attacks(radiant)`, whose test asserts `!Arc::ptr_eq`), the
faces hold two closures.

| Card | Hooks | Notes |
|---|---|---|
| `core-032` Prem Panther | `afterAttack` | `ctx:after_attack_of()`: `param("draw")` per destroyed id |
| `core-041` Sheepish | `triggers` (`when`, `run`) on `cardResolved` | held back by batch 7; `ctx:left_field_since_resolved(event)` (#464 gap 9); per-face ids `sheepish` / `sheepish-radiant` |
| `core-086` "Miss" Mrow | `death` | held back by batch 6; `ctx:killer_of_self()` (#464 gap 9), the Death's snapshot (`SELF_KEY`), as `killer_of(ctx.self_)` read it; `radiant = "base"` |
| `core-089` Corpse Eater | `handTriggers` on `destroyed` | `ctx:def_of(event.defId)` for type and token; `factor` per face |
| `core-092` Felinor Fiender | `setStat` | active plus dormant units, `tags_of` by id (R923), `stats_with_buffs` by id; multiple 1 / 2 |
| `classic-004` Palantir | `drawLimit`, `triggers` (`when`, `run`) on `cardAnnounced` | radiant `recalled("stolenCost")`, `own_cost(id) or 0`, `remember`, Tribute at `param("threshold")` |
| `classic-006` Cloaked Toe Cracker | `costAura`; radiant `triggers` on `cardPlayed` | |
| `classic-013` Boots on the Ground | `afterAttack` | radiant: one `recruit` per `param("recruits")` |
| `classic-027` Pestilent Slime | `plagueMultiplier` | `radiant = "base"` |
| `classic-028` Second Wind | `cry`, `graveyardPlay`; base `replacements` (`toGraveyard`, `when`) | the `when` reads `ctx:event().owner` and `ctx:self_().memory.secondWindCry` |
| `classic-047` Recurring Felinor | `cry` (`cast_new` of C #34 on its base face), `graveyardTriggers` on `trapFired` | radiant `costOverride = param("returnCost")` |
| `classic-049` Anti-Greed Machine | `drawLimit` | base `PLAYERS_LIMIT = 1` on both players; radiant `param("limit")` on the enemy |
| `classic-055` Book of Wildfire | `targets`, `cry`, `handTriggers` | the engine's `book_swap_trigger` written out (`api-gaps.md`, Declarations): id `book-swap` (`BOOK_SWAP_TRIGGER_ID`, which `book_swap::granted_hand_triggers` looks for), `swapsBook` read off `enchantments`; `radiant = "base"` |
| `classic-062` Living Bomb | `startOfTurn`, `startOfOpponentTurn` | one opponent-turn function in both faces (the test's `Arc::ptr_eq`) |
| `classic-066` EU Striker | `handTriggers` and `triggers` on `cardResolved` | `J.summon_this()`, `J.bounce`; `radiant = "base"` (the test's `Arc::ptr_eq` on each `run`) |
| `classic-068` Small Card Lobbyist | `costAura` | base threshold rung for all; radiant `ban = true` on opponents |
| `classic-074` Corpse Plantation | `activations` (`plant`, `uses = 1`), `graveyardPlay` | `radiant = "base"` |
| `classic-075` Argusland | `heroGuard` (`divisor`) | `radiant = "base"` |
| `classic-077` Anti-Magic Monkey | `costAura` | `radiant = "base"` |
| `classic-087` Plague Chalice | `cry`, `triggers` (`when`, `run`) on `cardAnnounced`, `wouldCounter` | the count is read off the instance (`counters.plague`, floored at 0), as `plague_on(me)` reads it |
| `classic-089` Paul Allen's Ghost | `targetingDiscards` | `radiant = "base"` |
| `classicplus-011` Anime Armor | `heroGuard` (`cap`) | `radiant = "base"` |
| `classicplus-019-2` Jungle Loser | `endOfTurn` (`for_each_card`, `with_kill_credit` with `pairs` / `then` by name) | held back by batch 6; `rng_chance(param("chance"), 100)` is the Rust `f64::from(chance) / 100.0` draw; base `kill_then` sends the Bot Loser Berserk, radiant `transfer = true` and no `then` |
| `classicplus-052` Jlockheed's Permanent Defense Contract | `cry` (`for_rest_of_game`), `delayed` | the label is `ctx:fill_params(ID, face, …)`: `find_def` of this id is the catalog's def, as `crate::card_def(ID)` is; `ctx:data()` carries the numbers (#464 gap 13) |
| `classicplus-073-1` Classic Golem | `afterAttack` | `transform_random` with `readyToAttack`; two closures (the test's `!Arc::ptr_eq`) |
| `classicplus-t-ai-02` Scaling Law | `setStat` | `played_this_game_with_tag(controller, "AI")`; `radiant = "base"` |

## 2. Cards that stay Rust

Each has its `.rs` exactly as at the base commit and no `.luau`.

- `classic-069` Plague Charger. Missing API (PORTING §5, reason 1; not in `api-gaps.md`): its
  aura raises only its own attack (`applies: unit.id == self.id`). The generic aura (#464 gap 8)
  takes a data filter (`side`, `rows`, `defId`, `exceptDefId`), which cannot name one instance,
  or a named `aura_filter` predicate, which is called with the candidate unit alone and cannot
  know the source. A `defId` filter would lend one Charger's bonus to every other Charger on
  that side. The fix is an instance field on `AuraFilter` (or the source passed to the
  predicate), in `crates/luau`, which this part may not touch. Its `conditionalKeywords` and
  `conditionMet` would port as they are; the card is one source.
- `classicplus-037` Wardrum. Missing API (reason 1; not in `api-gaps.md`): both end-of-turn
  hooks cast through `cast_new` with `CastNewDef::Read`, a definition read as the cast applies
  (base: the rng pick among this turn's non-Unit plays that still exist; radiant: each copy's
  definition and face read as that copy is cast, so an earlier copy's resolution that transforms
  a later played card, or makes it cease to exist, changes or skips it, R86). `J.cast_new` takes
  only a fixed `def` (an id or `{ defId, radiant }`), and `for_each_card`'s `each` has no
  context to read the card with, so a Luau Wardrum fixes every definition when the hook runs.
  The fix is a by-name `def` read (`def = "cast_def"`, called with the context as the cast
  applies). Its hand and deck triggers and `conditionMet` would port as they are.
- `classic-088` Siphon Squad. Breaks `luau bench` (reason 3). The port was faithful (200 games,
  0 mismatches, its tests green), but with it the bench measured `fuzz --seeds 200` at ×2.08 the
  parent's, over the ×2 budget (section 4). A throwaway timer around `vm::with_vm` and
  `call_aura_hook` (reverted, never committed) put Siphon Squad's `aura` at 34 s of CPU over
  231,566 calls by its last report in one `fuzz --seeds 200` (after 500,000 hook calls), about
  148 µs a call, the most of any hook; the next
  were Core #1's `aura` (already Luau at the base, about 113 µs a call, so the cost is mostly the
  call itself, not the script) and Felinor Fiender's `setStat` (5 s). Without Siphon Squad the
  bench passes. A Siphon Squad aura cheap enough needs a cheaper aura call in `crates/luau`.

## 3. The tests

No assertion changed. What changed in the 26 ported files' `mod tests`:

- a call of `script()` became `crate::scripts_of()[ID].clone()` (`super::super::script()` became
  `crate::scripts_of()[super::super::ID].clone()`), and `use super::{ID, script}` (or
  `use super::script`) became `use super::ID`, as batches 1 to 9 did;
- four files whose tests named `Arc` through the deleted `use jackioh_engine::prelude::*`
  (Pestilent Slime, Corpse Plantation, Argusland, Classic Golem) got `use std::sync::Arc;` in
  `mod tests`, as Classic #12, #31 and #76 did;
- Felinor Fiender and "Miss" Mrow lost a `use super::*;` that imported nothing once `script()`
  went (an unused import is a warning, and clippy runs `-D warnings`).

## 4. Checks

All on this machine (linux x86_64, 4 threads), the parent `jackioh` built from the base commit
before any change and copied out (`/tmp/parent462/jackioh`), in place of PORTING's
`git worktree add ../base`, so no second worktree was made.

- `cargo test -p jackioh-cards`: 5,860 unit tests and 752 cross-card tests passed, 0 failed.
  The ported cards' own tests (`cargo test -p jackioh-cards --lib -- <module>…`): 225 passed for
  the fourteen ported first, one per hook type (Siphon Squad's port among them), and 266 for the
  other thirteen.
- `cargo test -p jackioh-ai`: 325 passed, 0 failed (the determinization tests among them).
- `cargo test -p jackioh-tools`: 163 passed, 2 failed: `luau::surface::tests::luau_core_002_bigot` and
  `luau_reports_print_as_markdown_and_json`, which survey Core #2 Bigot's Rust script, gone since
  part 4 (#446). Neither reads a file this part touches, so they fail the same at the base.
- `cargo clippy -p jackioh-cards --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean on every file this part touches; it reports one diff, in
  `crates/luau/tests/api.rs` (part 28's file, line 1101), which this part may not touch.
- `cargo jackioh luau diff --parent-bin /tmp/parent462/jackioh`, 200 games each (seeds 1–200),
  on the final build. The ported cards do not fit one 20-card deck, so they ran in groups. The
  T-AI-2 token cannot be forced (no card's `refs` name it), so Classic+ #43 AI Slop and #78
  Claude's Datacenter, whose AI-token pools bring it in, were forced instead, as part 15 did:
  - A: `core-032, core-089, core-092, classic-004, classic-006, classic-013, classic-027,
    classic-028, classic-047, classic-049, classic-055, classic-062, classic-066, classic-068`:
    12,572 steps, **0 mismatches**;
  - B (Siphon Squad, Rust in the final build, stays in it): `classic-074, classic-075,
    classic-077, classic-087, classic-088, classic-089,
    classicplus-011, classicplus-052, classicplus-073-1 (through classicplus-073),
    classicplus-019-2 (through classicplus-019), core-086, core-041, classicplus-043,
    classicplus-078`: 29,445 steps, **0 mismatches**;
  - C, the three Rust stays among the ported cards they meet: `classic-069, classicplus-037,
    classic-087, classic-062, classic-027, classic-074, classic-006, classic-004, classic-049,
    classic-066, classic-047, core-041, classic-088, classic-028`: 14,141 steps,
    **0 mismatches**.
- What those games ran, folded from `golden record` of groups A and B by a throwaway counter
  outside the repository (plays / games with a play): Prem Panther 189 / 119, Corpse Eater
  99 / 80, Felinor Fiender 196 / 136, Palantir 196 / 132, Toe Cracker 187 / 128, Boots 234 / 136,
  Pestilent Slime 174 / 119, Second Wind 284 / 194, Recurring Felinor 194 / 130, Anti-Greed
  Machine 162 / 117, Book of Wildfire 181 / 102, Living Bomb 179 / 131, EU Striker 325 / 123,
  Lobbyist 136 / 119, Corpse Plantation 247 / 167, Argusland 271 / 176, Anti-Magic Monkey
  401 / 171, Plague Chalice 286 / 191, Paul Allen's Ghost 323 / 161, Anime Armor 335 / 163,
  Defense Contract 250 / 157, "Miss" Mrow 269 / 148, Sheepish 234 / 143 (206 firings), Scaling
  Law 133 / 87; Classic Golem summoned 21 times and transformed 10, Jungle Loser summoned 67.
- `cargo jackioh golden check`: 240 of 240 games replayed identically (22,713 steps).
- `cargo jackioh fuzz --seeds 200`: 200 passed, 0 failed, 0 panics, 0 replay mismatches, 0
  invariant violations.
- `cargo jackioh luau bench --parent-bin /tmp/parent462/jackioh`:
  - with Siphon Squad ported: fuzz 12,765 ms → 26,596 ms (×2.08, over budget), slowest decision
    113.3 ms → 135.1 ms (×1.19), golden step 1.830 ms → 3.232 ms (×1.77);
  - final (Siphon Squad Rust): fuzz 12,956 ms → 17,433 ms (×1.35), slowest decision 113.7 ms →
    115.4 ms (×1.01), golden step 1.861 ms → 2.365 ms (×1.27): **within every budget**.
- `cargo jackioh spec check`, `catalog check`, `patches check`: ok.
- `luau-analyze --mode=strict`: **not run**. The binary is not installed here, fetching the
  Luau 0.740 release was refused in this sandbox, and `luau0-src` (the copy `mlua` builds) carries
  no `Analysis` sources to build it from. The reconcile, or a run with the analyzer, should run
  it over the 26 new files; the ports annotate every hook context and hoist union-typed tables
  into annotated locals (`J.CostAura`, `J.DrawLimit`, `J.HeroGuard`, …) as PORTING §2 asks.

## 5. Decisions

- **Siphon Squad stays Rust on the bench, not the others.** Rejected: dropping Felinor Fiender or
  the cost auras too; without Siphon Squad the bench is at ×1.35, and each of them is correct.
- **Plague Charger and Wardrum stay Rust for gaps `api-gaps.md` does not list.** Rejected: a
  `defId` aura filter for the Charger (wrong with two Chargers) and Wardrum's definitions fixed
  when the hook runs (wrong once a copy changes a later card), both divergences `luau diff` might
  not meet in 200 games. New gaps are written here, not in `api-gaps.md`, as part 15 did.
- **The parent is a copied binary, not a second worktree.** It was built from the unmodified
  base tree before the first edit, which is the same oracle.
- **Arc identity follows the Rust wherever a test asks.** Where the Rust cloned one hook into
  both faces, the module shares one value or says `radiant = "base"`; where it built two
  (Recurring Felinor's and Second Wind's `cry`, Classic Golem's `afterAttack`), the faces hold two
  functions. One exception no test reads: Plague Chalice's `cry` has no upvalue, so Luau makes one
  closure for both faces and they share one adapter, where the Rust built two; the run is the
  same.
