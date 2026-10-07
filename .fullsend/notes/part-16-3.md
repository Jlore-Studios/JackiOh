# Slice: part 16 (cards lane 8: Classic+ #50–#78 and the AI tokens), chunk 3 of 3 (#404, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
None of the 17 existed on `staging` (no bot half-port; no `.fullsend/notes/part-16-bot.md`). All are full
ports: the TS script (header as `//!`, every function in TS order, its doc and rule comments) with the TS
test file as `#[cfg(test)] mod tests` (its header above, one `mod` per `describe`, one `#[test]` per `it`;
counts checked by grep against the TS files). No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/classic_plus/c074_twice_forward_one_step_backwards.rs` (24 tests)
- `…/c075_1_j_lease_j_jungle_ex_plorer_pack.rs` (11), `…/c075_j_lease_j_jungle_ex_plorer.rs` (9)
- `…/c076_1_brother_ping.rs` (17), `…/c076_brother_lar.rs` (10), `…/c077_anti_softlock.rs` (13)
- `…/c078_claudes_datacenter.rs` (11)
- `…/t_ai_01_helpful_assistant.rs` (10) … `…/t_ai_10_fine_tuning.rs` (8, 9, 15, 15, 12, 9, 11, 17, 9)

## SURFACE
§7.1 shape everywhere: `use jackioh_engine::prelude::*;` plus the effect verbs named from
`jackioh_engine::effects`, `pub const ID`, `pub fn script() -> CardScripts`, private helpers only.
Tests: `use jackioh_engine::testkit::*;` (no `use super::*` at the top, so the prelude's verbs never meet
the testkit's root globs); nested `mod`s `use super::*`; the card's own items as `super::super::ID` /
`super::super::script()`. Every test that builds a scenario calls `crate::register_all()` first.

## GAPS (names called in other parts' modules, with the shape assumed)
- Effects (parts 6/7), each one argument built with `json_as(json!({ …TS literal… }))`, so only the serde
  shape matters: `add_random_from_catalog` ({query, count, radiant, costOverride}), `shuffle_into`
  ({defId, count, radiant}), `damage` ({to: {of: "chosen"}, amount}), `summon` ({defId, radiant}),
  `draw` ({count}), `grant_keyword_cards` ({scope: {side, zones}, keyword}), `discover_from_library`
  ({step, prompt}), `add_to_hand` ({instance: {of: "chosen"}, costMod} and {defId, radiant, costOverride}),
  `add_library_copies` ({of, count, brittle}), `draw_while_cheap` ({maxCost, repeats}),
  `destroy_field_spells_and_hit` ({side, damagePer}), `add_cost_rule` ({player, rule: {amount}, lasts}),
  `end_turn` ({player}), `counter_play` ({target: {of: "instance", instanceId}}), `upgrade`
  ({scope: {zones}, random}). `unlock_all(Default::default())` (TS `args: ZoneScope = {}`).
- `effects::choose::chosen_options(&EffectContext) -> Vec<String>`.
- `effects::datacenter::{FieldSpellSide: Deserialize (built from "any"/"enemy" with json_as),
  SweepReader::of_condition(ConditionContext), field_spells_doomed(&SweepReader, FieldSpellSide) -> Vec<_>}`
  (part 6.2's notes).
- `effects::cast::{cast_new(CastNewArgs), CastNewArgs { def: CastNewDef, radiant: Option<bool>, how:
  CastHow: Default }, CastNewDef: From<&str>}` (T-AI-2's test; field names guessed from part 6.2's notes).
- `effects::summon::summon_copy(SummonCopyArgs: Deserialize)` ({of, lane}; C+ #75's test).
- `subsystems::twice_forward::{twice_forward_trigger(TwiceForwardArgs: Deserialize {radiantCopy}) ->
  TriggerDef, twice_forward_plays(&CardInstance) -> i32}`; `subsystems::fuse::fuse(&mut EngineSink,
  FuseArgs: Deserialize {ingredients, into, handPrice: "fused"}) -> Option<CardInstance>` (T-AI-2 test).
- Engine readers: `params::param(&impl ParamContext, &str) -> i32` called as `param(&*ctx, key)`;
  `params::step_param(&mut CardInstance, &str, i32)`; `query::{played_this_game_with_tag(&GameState,
  PlayerId, Tag) -> i32, cards_played_this_turn(&GameState, PlayerId) -> i32, last_face_up_played(
  &GameState, PlayerId) -> Option<FaceUpRecord { def_id: String, radiant: bool, .. }>}`;
  `zones::{active_units_of(..) -> Vec<&CardInstance> (or owned), beneath_at (slice or Vec: called as
  `&beneath_at(..)`), lock_zone(&mut GameState, ZoneRef), is_locked(&GameState, ZoneRef)}` (a `ZoneRef`
  built with `json_as`); `catalog::def_of(Option<&GameState>, &str) -> &CardDef` (part 2.2's notes);
  `brittle_count::active_brittle_count(&CardInstance) -> Option<i32>`; `draw::draws_this_turn(&GameState,
  PlayerId)`; `mana::effective_cost(&GameState, &CardInstance, Default::default())`;
  `view_for::HIDDEN_ID`; `reduce`, `hash_state`, `legal_actions`.
- Test plumbing: `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions {
  controller: Option<PlayerId>, .. }: Default), apply_effects(&[Effect], &mut EffectContext)}`,
  `triggers::{settle(&mut EngineSink, SettleOptions::default())}`; a `Hook` called as `hook(&mut ctx)`.
- Testkit (part 5.1's notes): `scenario(Value)`, steps returning `&mut Scenario`, `state_mut()`,
  `unit`/`backrow -> Option<CardInstance>`, `hand`/`pile -> Vec<CardInstance>`, `card(ref) ->
  &CardInstance`, `expect_refused(|s| s.step(..))`, `expect_events(Value array)`; refs as `&str` (def id or
  instance id) or `&CardInstance`.
- `crate::query::query(&CardQuery)` (C+ #78's test), `crate::card_def(&str) -> CardDef`.

## Decisions
- TS `cardDef("<token>").id` used as a value (C+ #75's Pack, C+ #76's Ping) is `crate::card_def(..).id`,
  read once in `script()` and moved into the faces' closures (no static).
- TS `{ base, radiant } as const` number pairs (T-AI-4, -6, -7, -8) are a private `Copy` struct per file
  with a `const`; C+ #78's `AI_POOL` object is a private fn returning the JSON.
- TS `radiant = base` (T-AI-2) is `base.clone()`; its `toBe` test checks `Arc::ptr_eq` on the hook.
- T-AI-6's `formulaIn` regex (`/Deal \d+ damage to [^.]+ for each one destroyed/`) is a hand matcher
  with the regex's leftmost/greedy semantics; panics with TS's message when absent.
- T-AI-9's `reaches` (TS `typeof yourUnit`) is a private `fn` pointer type, so `when` and `run` both carry it.
- Tests: TS live-object writes (`s.card(X).tuning = …`, `stepParam(s.card(X), …)`, `s.state.… = …`, the
  original library card's fields) go through `find_instance_mut(s.state_mut(), &id)`; TS `sinkOf(s)` /
  direct engine calls build an `EngineSink` over `s.state_mut()` and write the rng cursor back.
- Tests compare wire data as JSON (`serde_json::to_value`): defs' cost/params/keywords/tags, zones,
  views, events' fields, modifiers' `kind`, prompt kinds; TS `toMatchObject` checks only the listed keys.
- TS spreads of side setups (`{ ...base, ...opts.p1 }`) are a private `spread` over the JSON objects.
- Test names: titles snake_cased, `§` → `s`, `#` → `n`, R-ids leading as `r<n>` tokens; the outer
  `describe` is a `mod` (`c_n74_…`, `t_ai_1_…`), its `base`/`radiant` describes nested `mod`s.
