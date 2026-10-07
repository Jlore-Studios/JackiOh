# Slice: part 12 (cards lane 4), chunk 4 of 4 (#400, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All seven were absent on `staging` (no night-bot half port, no `part-12-bot` notes); all are full ports,
script and tests, every TS function in TS order with its header and rule comments (R-ids, §-ids kept).
One `#[test]` per TS `it` (counts checked: 20, 15, 14, 18, 20, 17, 16), one nested `mod` per `describe`.
No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/classic/c028_second_wind.rs` ← `classic/028-second-wind.ts` + test
- `crates/cards/src/scripts/classic/c029_book_of_vital_kill.rs` ← `classic/029-book-of-vital-kill.ts` + test
- `crates/cards/src/scripts/classic/c030_recycle.rs` ← `classic/030-recycle.ts` + test
- `crates/cards/src/scripts/classic/c031_cookie_guild.rs` ← `classic/031-cookie-guild.ts` + test
- `crates/cards/src/scripts/classic/c032_felinor_feelings.rs` ← `classic/032-felinor-feelings.ts` + test
- `crates/cards/src/scripts/core/t_rush.rs` ← `t-rush.ts` + test
- `crates/cards/src/scripts/core/t_sheep.rs` ← `t-sheep.ts` + test
Files left: none.

## SURFACE
Each file: `pub const ID`, `pub fn script() -> CardScripts`, private helpers, `#[cfg(test)] mod tests`
(SURFACE §7.1, §7.3). Part 1's frozen types used as compiled (hooks take `&mut EffectContext`; typed
hooks take their `Copy` argument bundle by value: `HookArgs`, `TargetCheckArgs`, `ReplacementContext`).

## GAPS (names called in other parts' modules, with the shape assumed)
Script half:
- `params::param(&impl ParamContext, &str) -> i32` (part 2.1): called as `param(&*ctx, …)` on a hook's
  `&mut EffectContext` and `param(&args, …)` on a `HookArgs`.
- `query::zone_cards(&GameState, PlayerId, zones::OffFieldZone) -> Vec<_>` (part 2.2; TS `OffFieldZone`).
  Only `.iter()` and `card.id` are used, so `Vec<CardInstance>` or `Vec<&CardInstance>` both compile.
- `zones::active_units_of(&GameState, PlayerId)` (`.iter()`, either element form), `mana::cost_now(&GameState,
  &CardInstance) -> i32` (part 4.1), `wire::opponent_of` (part 1).
- Effects (parts 6–7), every data argument built with `json_as(json!(…TS literal…))`, so only the serde
  shape matters: `remember`, `exile_matching`, `set_health`, `add_to_hand`, `draw`, `shuffle_card_into`,
  `set_cost_mod`, `recruit`, `steal`, `summon`; `discard_hand(Default::default())` (TS `discardHand()`).
- `effects::each::{for_each_card, ForEachCardArgs { cards, each }}` built by struct literal with
  unannotated closures (`Arc::new(|c| …)`, `Arc::new(|instance_id| …)`), so the closure signature is
  taken from the field types: `cards` must answer `Vec<String>` (part 6.2's notes) and be callable with
  either `&EffectContext` or `&mut EffectContext`; `each` gets the id as `&str` or `String`.
- `effects::choose_where::{choose_target_where, ChooseTargetWhereArgs}`: built with `json_as` from
  `{ step, scope, prompt }` (so it must `Deserialize`, part 6.1 says it does), then
  `args.where_ = Some(Arc::new(|ctx, card| …))` with `card: Option<&CardInstance>` (TS `CardInstance | null`).
- C #28's replacement `when` reads `ctx.self_.memory[CRY_RUNNING]` through a private copy of
  `query::recalled` (TS called `recalled({ self, data: {} }, key)`, which with empty data is the key
  itself): `query::recalled` takes an `EffectContext` (part 2.2), which a pure-read replacement has none of.
Test half (the testkit is part 5.1's, as its notes give it):
- `scenario(Value)`, `Scenario::{play, attack, answer, end_turn, start_turn, expect_*, state, state_mut,
  events, last_events, view, unit, backrow, hand, pile, card}`; free fns `expect_throw(|| …)` and
  `expect_throw_with(|| …, "text")` (part 5.1's names for a refusal that is not a step). Every test calls
  `crate::register_all()` first (part 5.1's instruction).
- `resolve::{make_context(&mut EngineSink, None, HookOptions { controller: Some(P1), ..Default::default() }),
  apply_effects(&[Effect], &mut EffectContext)}` (part 3.2's shape) in t_rush's `summon_rush_token`.
- `catalog::{query(&CatalogQueryArgs) -> Vec<&CardDef>, def_of(Option<&GameState>, &str) -> &CardDef}`
  (part 2.2's own shapes: `def_of` takes `Option<&GameState>`).
- `layers::{unit_view, unit_has(&GameState, &CardInstance, KeywordKind), keywords_of -> Vec<Keyword>,
  face_of -> FaceStats (Serialize, camelCase {attack, health, keywords})}`.
- `zones::{move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, Default::default()) -> MoveResult,
  MoveResult::Vanished (PartialEq + Debug), is_unit_token, card_at(&GameState, &ZoneRef) -> Option<&CardInstance>}`.
- `play_choices::{tribute_value_of(&GameState, &CardInstance) -> i32, legal_tribute_units(&GameState,
  PlayerId, &CardInstance), tribute_cost_of(&GameState, &CardInstance) -> i32}` (part 4.2: the state added).
- `query::{hero_of(..).armor, unspent_mana_of}`, `params::step_param(&mut CardInstance, &str, i32)`,
  `reduce::{reduce, legal_actions}`, `state::{find_instance, find_instance_mut}`.

## Decisions
- No night-bot work existed for these seven paths, so all are written fresh.
- TS shared constants (`const cry`, `base.targets`) are private fns or locals cloned into both faces;
  `radiant = base` is `base.clone()` (t_rush, c031), and c031's `toBe(base)` test checks
  `Arc::ptr_eq` on the two Cries (a clone shares the `Arc`).
- C #30's Radiant discount: TS read `-param(ctx, "discount")` lazily inside `each`; Rust reads it once in
  the Cry, where the context is in hand (a `'static` closure cannot hold the context). The number cannot
  move between the two moments.
- TS `toEqual({})` / `toEqual({ staticFlags: … })` on a Script: a private field-by-field check over all
  of `Script`'s fields (Script derives neither `PartialEq` nor `Debug`).
- TS test helpers that wrote through the live instance (`stepParam(s.card(x), …)`, `delete billy.x`)
  write through `find_instance_mut(s.state_mut(), id)`.
- Wire comparisons against TS literals (events, views, params, declarations) go through
  `serde_json::to_value`; `JSON.stringify(view)` is `serde_json::to_string`.
- Test names: title lower-cased, `§` → `s`, `#` → `n`, other runs → `_`; ruling tokens lead.
