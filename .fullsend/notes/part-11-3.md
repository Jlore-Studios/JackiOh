# Slice: part 11 (cards lane 3), chunk 3 of 4 (#399, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All five were absent on `staging` (no night-bot half-port to finish); all are full ports, script and
tests, every TS function and every `it` (095: 41, 096: 23 `it`s → 26 tests, the R662 loop written out
per face, 097: 19, 098: 35, 099: 23). No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/core/c095_call_to_chaos.rs` ← `095-call-to-chaos.ts` + `.test.ts` (the test file's
  #95.1 Chaos Golem `describe` included)
- `crates/cards/src/scripts/core/c096_my_pawn.rs` ← `096-my-pawn.ts` + `.test.ts`
- `crates/cards/src/scripts/core/c097_zephyrs.rs` ← `097-zephyrs.ts` + `.test.ts`
- `crates/cards/src/scripts/core/c098_heroic_power.rs` ← `098-heroic-power.ts` + `.test.ts`
- `crates/cards/src/scripts/core/c099_craft_a_card.rs` ← `099-craft-a-card.ts` + `.test.ts`

## SURFACE
§7.1 shape in every file: `//!` header, `use jackioh_engine::prelude::*;` (plus the effect verbs named
explicitly), `pub const ID`, `pub fn script() -> CardScripts`, tests in `#[cfg(test)] mod tests` with
`use super::*; use jackioh_engine::testkit::*;`, one nested `mod` per `describe`, names by §7.3, every
test starting with `crate::register_all()`. Part 1's frozen code where it differs from SURFACE: hooks
take `&mut EffectContext`; `TriggerDef.when` is `(&EffectContext, &GameEvent) -> bool`, built here
with `TriggerDef::new(..)` and `when` set to a shared `TriggerWhen`; `ConditionHook` by value
(`condition_hook(fn)`).

## GAPS
Nothing left out. Names called in other parts' modules (TS name snake_cased at its TS module's path),
with the shape assumed:
- part 8.2 `subsystems::call_to_chaos`: `call_to_chaos(CallToChaosArgs { radiant: Option<bool>, table:
  Option<&'static [ChaosEffectDef]> })` (written as a literal with exactly those two fields),
  `roll_chaos_effects(&mut Rng, bool, None)` (third argument `Option<&[ChaosEffectDef]>`, as part 8.2's
  notes say; parts 22.3/25.1 assumed other arities) returning a `Vec` of entries whose `.name` and
  `.label` are `&'static str` (read with `.to_string()`), `CHAOS_EFFECTS` (iterable), `CHAOS_CHAIN_KEY: &str`.
- part 8.2 `subsystems::lethal`: `defending_hero(&AttackTarget) -> PlayerId`, `is_lethal(&GameState,
  &CardInstance, &AttackTarget) -> bool`. Part 3.1 `combat::attack_target_of(&GameState, &str) ->
  Option<AttackTarget>`. Part 2.2 `query::lethal_attackers_of(&GameState, PlayerId)` (anything with
  `.is_empty()`).
- part 8.2 `subsystems::scorer`: `top_three`/`rank(&GameState, PlayerId, &ScorerOptions) -> Vec<Scored>`,
  `ScorerOptions { radiant: Option<bool> }` (one field, built as a literal), `Scored { def, priority }`
  (`def.id`; `priority: Serialize`, compared as JSON `"lethal"`/`"clear"`), `candidate_defs()` (iterable of
  defs with `.id`, serialisable).
- part 8.2 `subsystems::hero_power`: `roll_power() -> Effect`, `power_abilities(bool) -> Vec<ActivationDecl>`,
  `POWER_RESUME: &'static str` (a `resume` key), `POWER_KEY: &str`, `hero_power::hero_power(&mut
  EffectContext) -> Vec<Effect>` (wrapped in `hook(..)`), `HERO_POWERS` (`.iter()`, entries with `x: i32`,
  `title`, `label`, `radiant_title`, `radiant_label`: Display), `HERO_POWER_NAMES` (Serialize, read as
  `&*HERO_POWER_NAMES` so a const slice or a `LazyLock` both work), `power_of(&CardInstance) -> Option<_>`
  with `.name: Serialize`, `STITCHING_MAX_COST: i32` (config, re-exported).
- part 8.1 `subsystems::fuse::fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>` (a borrowed result
  also compiles: it is `.clone()`d), `FuseArgs: Deserialize` (built with `json_as` from TS's literal, the
  instances as their JSON); `subsystems::FUSE_MIN_INGREDIENTS: usize`, `CRAFTED_CARD_COST: i32`.
- part 6.1 effects: `discover_from_catalog(DiscoverFromCatalogArgs)` — fields deserialised from TS's
  literal (`step`, `query`, `count`, `prompt`, `data`), and `query_fn: Option<Arc<dyn Fn(&EffectContext)
  -> CatalogQueryArgs + Send + Sync>>` set in Rust (#97; the closure's argument type is left to inference,
  so `&mut EffectContext` works too); `cancel_attack(<{ destroyAttacker? }>)`, `ai_plays_out_turn(<{ player,
  settleFirst? }>)` built with `json_as`; `chosen_options(&EffectContext) -> Vec<String>`. Part 6/7:
  `exile(<{ target: { of: "self" } }>)`, `add_to_hand(<{ defId, radiant }>)`, `fuse_cards(<{ defIds,
  toHand: "self" }>)`, all through `json_as`.
- part 3.2 `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext,
  HookOptions: Default { controller: Option<PlayerId>, .. }}` (#96's `arms`).
- part 4.1 `mana::{effective_cost(&GameState, &CardInstance, CostOptions: Default), printed_cost(&GameState,
  &CardInstance) -> i32}`; part 2.2 `catalog::{query_cost(&CardDef) -> i32, CatalogQueryArgs: Deserialize}`;
  part 5.2 `legal_actions(&GameState, PlayerId) -> Vec<ActionBody>`.
- part 9 `crate::query::query(&CardQuery)` (iterable of defs with `.id`), called as
  `crate::query::query(&json_as(json!({ … })))`.
- part 11 chunk 2 `crate::scripts::core::c095_1_chaos_golem::script()` (the generated module path), for
  the #95.1 tests that live in #95's TS test file.
- part 5 testkit: the `Scenario` API of part 5.1's notes (`unit`/`backrow` take `lane: i32` and return
  copies, `card()` resolves instance ids too, assertions return `&mut Scenario`, `expect_refused(_with)`
  closures return the `&mut Scenario`); `glow::{backrow_glows, opponent_sees_glow}(&Scenario, lane,
  PlayerId) -> bool` (part 5.3).

## Decisions
- #95's two faces are private `fn base()`/`fn radiant()` so TS's doc comments stay doc comments (a `///`
  on a `let` is rustc's `unused_doc_comments`).
- #96: TS handed both faces' triggers the same `wouldBeLethal` object, and a test asserts it
  (`toBe`). `script()` builds one `TriggerWhen` (`Arc::new(would_be_lethal)`) and `my_pawn(destroys_attacker,
  &when)` takes it as a second argument; the test checks `Arc::ptr_eq`. `radiant).not.toBe(base)` is
  "the two triggers' `run`s are different `Arc`s".
- Tests read events, views, pending prompts, legal actions and catalog defs as JSON
  (`serde_json::to_value`), as TS compared plain objects: `toMatchObject` is a private `matches_object`
  (arrays item by item, as vitest does), `toEqual` on objects is `==` on `Value`. TS's `Object.keys(script)`
  is a private `members(&Script)` listing the TS keys of the members a face sets (every `Script` field of
  part 1's freeze).
- A TS test's write through a live instance (`s.card(x).memory[…] = …`, `.radiant = true`,
  `.grantedKeywords = …`, `.divineShieldSpent = true`) is `find_instance_mut(s.state_mut(), id)`; the
  harness's reads are copies, so a card is re-read after each step.
- #96's `arms` builds its sink over a clone of the scenario's state (the predicate only reads).
- #99's `sinkFor`: a Rust sink borrows its state, events and rng, so each Fuse test builds it inline in a
  block over `s.state_mut()` and checks the events `Vec` after the block (TS read `sink.events`).
- `findIndex`'s -1 is kept as `isize` where TS compared indexes, so a missing event fails the same way.
- `expect(() => s.play(…)).not.toThrow()` is the plain call (a panic fails the test).
- The #95.1 golem's `golemRadiant).toBe(golemBase)` and `toEqual({})` become "both faces list no members".
- #99's `craft_a_card(discovers: i32)` takes TS's `2 | 3` as a number; the resume table is built in TS's
  key order (`first`, `second`, then `third` on the radiant face).
- Comments that named TS-only mechanics (the `subsystems.CALL_TO_CHAOS_CHAIN_CAP` pitfall, TS file paths)
  are reworded for Rust; every rule and ruling they cite is kept.
