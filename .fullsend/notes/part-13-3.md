# Slice: part 13, chunk 3 of 4 (cards lane 5: Classic #54–#63; #401, parent #306)
BUILDS-RUN: 0

## FILES
Each holds `pub const ID`, `pub fn script()` and a `#[cfg(test)] mod tests` with one `#[test]` per TS `it`
(one `mod` per `describe`), the TS headers kept (`//!` above the script, `//` above the tests):
- new: `crates/cards/src/scripts/classic/{c054_rewind, c055_book_of_wildfire, c056_spell_tyrant,
  c061_plague_bringer_goliath, c062_living_bomb, c063_crop_dusting}.rs`
- the night bot's half-done ports (commit 0180138), read against both TS files and finished in place:
  `crates/cards/src/scripts/classic/{c057_echo, c058_common_resources, c059_plague_doctor, c060_pile_on}.rs`
  (every TS `it` was there; the fixes are under Decisions).
No `todo!`, `unimplemented!` or `// TODO`. Nothing left out.

## SURFACE
§4.1 paths, §7.1 shape (`use jackioh_engine::prelude::*;`, explicit `use jackioh_engine::effects::{…}` for
the verbs), §7.3 test names, §8 testkit verbs as part 5.1's notes give them. Hooks take `&mut EffectContext`
and every pure-read hook its argument bundle by value (part 1 over SURFACE §6.6).

## GAPS
Names called at their TS module's path, with the shape assumed:
- `effects::cry::{trigger_cry(TriggerCryArgs: Default), has_triggerable_cry(&GameState, &CardInstance) -> bool}` (part 6.1).
- `effects::cast::{cast_each(CastEachArgs) -> Effect, CastEachArgs { cards: CastEachCards, how: CastHow }}` with
  `CastEachCards = Arc<dyn Fn(&EffectContext) -> Vec<String> + Send + Sync>` and `CastHow: Deserialize`
  (`{ "afterward": "exile" }`) (part 6.2, as part 24-7's notes report it). One private helper in
  `c056_spell_tyrant.rs` (`cast_each_then_exile`) is the only place that names the struct.
- Data verbs built with `json_as(json!(TS literal))`: `choose_pick`, `damage`, `destroy`, `draw`,
  `draw_from_opponent`, `place_plague`, `place_plague_each`, `place_plague_tokens`; `recruit_all(Default::default())`,
  `trigger_cry(Default::default())` (TS `args = {}`).
- `book_swap::{book_swap_trigger() -> TriggerDef, BOOK_SWAP_TRIGGER_ID}` (part 2.1's fn in place of the TS constant).
- `params::{param(&impl ParamContext, &str) -> i32, step_param(&mut CardInstance, &str, i32)}`;
  `tuning::{tuning_of(&mut CardInstance) -> &mut Tuning, add_step(Option<&IndexMap<String, i32>>, &str, i32) -> IndexMap<String, i32>}`.
- `plague::{permanents_on_field(&GameState, PlayerId) (a bare PlayerId also fits `impl Into<Option<PlayerId>>`),
  plague_on(&CardInstance) -> i32, plague_multiplier_of(&GameState, &CardInstance) -> i32}`; the list's items
  may be owned or borrowed (read only through auto-deref).
- `query::{zone_cards(&GameState, PlayerId, OffFieldZone), cards_played_this_turn, played_this_game_with_tag(&GameState,
  PlayerId, Tag) -> i32, last_spell_played(&GameState) -> Option<PlayRecord>, hero_of(..).health}`;
  `zones::OffFieldZone::Graveyard` (TS `zones.ts`'s union, not defined by part 1); `faces::card_type_of(&GameState,
  &CardInstance) -> CardType`; `draw::draws_this_turn(&GameState, PlayerId) -> i32` (root re-export).
- `subsystems::copied_text_of(&GameState, &CardInstance) -> Option<PlayRecord>` (part 8.2, globbed by part 1's mod.rs).
- `effects::tune::{applicable_changes(&GameState, &CardInstance, TuneDirection) -> Vec<TuneRow>, TuneDirection::{Upgrade,
  Degrade}}`, `TuneRow: Serialize` as its TS name (compared as JSON).
- `catalog::query(&CatalogQueryArgs)` (`CatalogQueryArgs: Deserialize`; items owned or borrowed).
- Testkit (part 5): `expect_refused(_with)` closures are `|s| s.play(..)` (part 5.1's `FnOnce(&mut Scenario) ->
  &mut Scenario`); `attack(&CardInstance, &CardInstance | "hero")`; `unit`/`backrow` return `Option<CardInstance>`;
  `answer(Value)` takes an id, a def id, `"hero:p2"`, a list of ids or a selection list as TS's harness did.
- `GameState`, `GameEvent`: `PartialEq + Debug` (tests `assert_eq!` states and event lists, as TS `toEqual`).
- `crate::card_def(id) -> CardDef` (part 1's cards `lib.rs`).

## Decisions
- The bot's ports, fixed: hook fns now take `&mut EffectContext<'_>` (c058, c059); Echo's `records_play_as` is a
  `read_hook` over `HookArgs` by value (the bot's `SelfArgs` does not exist); Plague Doctor's preview is a
  `condition_hook` over `ConditionContext` by value and its `Read` is built from either context;
  `permanents_on_field` is passed a bare `PlayerId`; Pile On's replacement is a `ReplacementDef` literal (it holds a
  `when` hook, so it is not `Deserialize`) and its test compares the declaration field by field; the testkit has no
  `card_mut`, so a private `step` helper writes through `find_instance_mut(s.state_mut(), id)` (and Echo's tuning
  likewise); `expect_refused` closures return the step; `backrow` copies are read through `as_ref()`;
  `assert_eq!(x, true|false)` became `assert!` (clippy `bool_assert_comparison` under `-D warnings`); test names
  starting with a section now start `s<n>_` (the bot wrote `c<n>_`).
- A TS JSON round trip (`JSON.parse(JSON.stringify(state))`) is `serde_json::to_string` then `from_str`, never
  `to_value`/`from_value`: without `preserve_order` a `Value` object sorts its keys, which would reorder every
  `IndexMap` in the state.
- TS `expect(radiant).toBe(base)` (one object): the Radiant face is `base.clone()`, and the test compares the
  declarations and which hooks are present. Where TS compares one shared hook (`radiant.cry toBe base.cry`,
  Living Bomb's `startOfOpponentTurn`), the `Arc` is shared and the test uses `Arc::ptr_eq`.
- `TriggerDef` is not data: Book of Wildfire's `toEqual([BOOK_SWAP_TRIGGER])` compares id (and `BOOK_SWAP_TRIGGER_ID`)
  and `on`; Crop Dusting's `[id, on]` pairs are compared as JSON.
- Engine values in tests are compared as their JSON (`js`) wherever TS compares a literal, so the assertions pin the
  wire shape. TS `toMatchObject` is a private `matches_object` (c055).
- Rewind's factory takes the wire's `FilterSide`; Living Bomb's two hooks are factory fns, the opponent's built once
  and shared by both faces; Crop Dusting's trigger is a `dusting()` fn (a closure cannot be a `const`); Goliath keeps
  TS's private `TRIBUTE_COST` const.
- Seeds, nonces and the TS loop bounds (60 and 20 swap seeds, up to 200 for Book of Flame) are TS's.
