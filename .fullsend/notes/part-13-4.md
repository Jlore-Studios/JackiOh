# Slice: part 13 (cards lane 5: Classic #33–#68), chunk 4 of 4 — C #64–#68
BUILDS-RUN: 0

## FILES
- `crates/cards/src/scripts/classic/c064_malzahars_recycler.rs` (the night bot's half-done port, commit
  0180138, finished in place: hooks and triggers rewritten to part 1's signatures, `card_mut` removed)
- `crates/cards/src/scripts/classic/c065_ace_in_the_hole.rs` (the night bot's half-done port, finished in
  place, same fixes)
- `crates/cards/src/scripts/classic/c066_eu_striker.rs` (new)
- `crates/cards/src/scripts/classic/c067_felinor_feeler.rs` (new)
- `crates/cards/src/scripts/classic/c068_small_card_lobbyist.rs` (new)

Every TS `it` is a `#[test]` (20, 20, 19, 9, 22), one nested `mod` per `describe`. No `todo!`,
`unimplemented!` or `// TODO`.

## SURFACE
Each file: `pub const ID`, `pub fn script() -> CardScripts`, `#[cfg(test)] mod tests`. Nothing else public.
Part 1's frozen `script.rs` wins over SURFACE §6.6 (hooks take `&mut EffectContext`; `TriggerDef::new(id,
&[types], run).with_when(when)` with `run(&mut EffectContext, &GameEvent)` and `when(&EffectContext,
&GameEvent)`; `read_hook` for the cost aura).

## DEPENDS-ON (names called that other parts provide; TS name snake_cased at its TS module's path)
- `jackioh_engine::query::{zone_count(&GameState, PlayerId, OffFieldZone) -> i32,
  recalled(&EffectContext, &str) -> Option<Value> (or Option<&Value>: both compile)}` (part 2).
- `jackioh_engine::zones::{OffFieldZone::Library, active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>
  (owned also compiles), beneath_at(&GameState, ZoneRef) -> &[CardInstance] or Vec (both compile)}` (part 2).
- `jackioh_engine::catalog::def_of(Option<&GameState>, &str) -> &CardDef or CardDef` (part 2.2's form; TS
  `defOf(state, defId)`).
- `jackioh_engine::params::{param(&impl ParamContext, &str) -> i32` for `EffectContext` (`param(&*ctx, …)`)
  and `HookArgs` (`param(&args, …)`), `step_param(&mut CardInstance, &str, i32)}` (part 2.1).
- `jackioh_engine::reduce::{reduce, legal_actions}`, `state::find_instance_mut` (part 1 / part 5).
- Effects (parts 6–7), every argument built with `json_as(json!(…))` from TS's literal (TS `= {}` as
  `json_as(json!({}))`): `move_::{discard_random, bounce}`, `draw::draw`, `memory::remember`,
  `summon::recruit`, `reveal::reveal`, `summon_this::summon_this()` (no argument, as TS),
  `position::switch_position_of`, `each::{for_each_card, ForEachCardArgs { cards: Arc<dyn Fn(&mut
  EffectContext) -> Vec<String> + Send + Sync>, each: Arc<dyn Fn(&str) -> Effect + Send + Sync> }}` (part
  6.2's note: ids, not instances).
- `jackioh_cards::query::query(&CatalogQueryArgs) -> Vec<CardDef>` (part 9; TS `catalog.query(...)`).
- Testkit (part 5): `scenario`, `Scenario::{play, attack, answer, end_turn, activate, state, state_mut,
  events, last_events, view, unit (Option<CardInstance>, owned), hand/pile (Vec<CardInstance>), card,
  stats (layers::UnitView with `attack`, `health`, `keywords`, `position`), expect_in_zone, expect_stats,
  expect_events, expect_mana, expect_refused, expect_refused_with}`.

## GAPS
- `effects::each::ForEachCardArgs.cards` (C #67): written as a closure over `&mut EffectContext<'_>`
  (part 24.1's note and the hook convention); part 24.3's note writes `&EffectContext`. If part 6.2 took
  `&EffectContext`, change the one closure's annotation in `c067_felinor_feeler.rs`.
- `jackioh_cards::query::query` (C #67's test): called as `query(&CatalogQueryArgs)`; if part 9 takes the
  args by value (or names its alias `CardQuery`), drop the `&` / rename at that one call.
- `testkit::Scenario` has no `card_mut`: the bot's `s.card_mut(…)` (C #64, C #65) became
  `find_instance_mut(s.state_mut(), &id)` — TS wrote through the live `s.card(…)` object.
- `layers::UnitView.position` is compared through its JSON (`"ATK"`/`"DEF"`), so `Position` or
  `Option<Position>` both work.

## Decisions
- The bot's ports of C #64 and C #65 were kept in substance and finished: every hook and trigger now takes
  `&mut EffectContext` (part 1), triggers are built with `TriggerDef::new(…).with_when(…)`, C #64's
  `answer` takes a `Hook` rather than a `fn` pointer (SURFACE §16 `hook.style`), and `recruit()` /
  `reveal()` take `json_as(json!({}))` (serde reads a missing `Option` as `None`).
- C #65's `fire` is a function returning the `TriggerDef` (a closure cannot be a `const`).
- C #66's `yourOtherPlay` returns `Option<&GameEvent>` (always the `CardResolved` variant), TS's
  `Resolved | null`. Its `expect(radiant).toBe(base)` is ported as `Arc::ptr_eq` on the triggers' `run`
  closures: `radiant` is `base.clone()`, so they are the same closures.
- C #68's cost auras are `CostAura` struct literals (part 1's type) through `read_hook`.
- TS `def` (from the script file) is `crate::card_def(ID)` read as JSON; the TS `toMatchObject` checks are
  a private `matches_object` (C #68) or field checks on the JSON.
- Test names: TS titles lower-cased, `§` → `s`, ruling tokens leading as the title has them
  (`s6_1_r638_…`, `r58_s2_4_…`); one `mod` per `describe` (`c_64_malzahars_recycler`, `base`, `radiant`).
- Every `#[test]` calls `crate::register_all()` first (part 5's note).
