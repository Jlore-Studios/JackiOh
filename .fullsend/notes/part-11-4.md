# Slice: part 11 (cards lane 3), chunk 4 of 4 (#399): Core #100 and the Core tokens Bread, Coin, Felinor, Ghoul
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All five hold the whole TS script (every function in TS order, its doc comments and every comment that
states a rule or cites a ruling) and, below it, the whole TS test file as `#[cfg(test)] mod tests` (one
`mod` per `describe`, one `#[test]` per `it`; a TS `for` over faces is one `#[test]` per face). No
`todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/core/c100_ceaseless_void.rs` (new) ← `100-ceaseless-void.ts` + its test (25 tests).
- `crates/cards/src/scripts/core/t_bread.rs` (new) ← `t-bread.ts` + its test (16 tests).
- `crates/cards/src/scripts/core/t_coin.rs` (new) ← `t-coin.ts` + its test (20 tests).
- `crates/cards/src/scripts/core/t_felinor.rs` (new) ← `t-felinor.ts` + its test (14 tests).
- `crates/cards/src/scripts/core/t_ghoul.rs` (the night bot's half-done port, finished in place; 13 tests).
  Fixed: `s.unit(..)` returns an owned copy (part 5.1's testkit), so the bot's `.cloned()` calls are gone;
  the effect runner builds its context with `EngineSink::new` and `HookOptions { controller: Some(P1), .. }`
  (part 3.2's `make_context`) instead of `json_as`; "§7 needs no script" now checks every `Script` field.

## SURFACE
Every file: `pub const ID: &str` and `pub fn script() -> CardScripts` (SURFACE §7.1); nothing else public.
Where part 1's frozen code or notes differ from SURFACE, part 1's are used: hooks take `&mut EffectContext`,
typed hooks are built with `cost_hook`/`condition_hook` over the `Copy` argument bundles.

## DEPENDS-ON (names called, expected from other parts, with the shapes assumed)
Script half (through `jackioh_engine::prelude::*`):
- `catalog::query_cost(&CardDef) -> i32` (part 2.2); `mana::play_cost(&GameState, &CardInstance) -> i32`
  (part 4); `query::unspent_mana_of(&GameState, PlayerId) -> i32` (part 2.2);
  `effects::exile_all(<BoardScope args>: Deserialize)` (part 7, TS `effects/move.ts`'s `exileAll`, args
  `{ side: "any", rows: ["units", "backrow"], excludeSelf: true }`); `effects::gain_mana(<args>:
  Deserialize)` with `{ amount }` (part 7).
- `crate::card_def(id) -> CardDef` (part 1's cards `lib.rs`).
Test half (named by module path, so no glob ambiguity can hide them):
- testkit (part 5.1, 5.3): `scenario`, `Scenario::{play, attack, answer, end_turn, start_turn, state,
  state_mut, events, last_events, view, unit, backrow, hand, pile, card, stats, expect_in_zone,
  expect_stats, expect_events, expect_health, expect_mana, expect_refused, expect_refused_with}`, the free
  `expect_throw_with(|| …, "text")`, `glow::hand_glows(&Scenario, &str, PlayerId) -> bool`.
- `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext,
  HookOptions { controller: Option<PlayerId>, .. }: Default, apply_effects(&[Effect], &mut EffectContext)}`
  (part 3.2).
- `mana::{printed_cost(&GameState, &CardInstance) -> i32, effective_cost(&GameState, &CardInstance,
  CostOptions: Default) -> i32}` (part 4).
- `catalog::{def_of(Option<&GameState>, &str) -> <a CardDef or &CardDef>, query(&CatalogQueryArgs) ->
  Vec<&'static CardDef>}` with `CatalogQueryArgs: Deserialize` from TS's literal (part 2.2).
- `layers::{face_of -> FaceStats { attack, health, keywords }, stats_with_buffs -> BuffedStats { attack,
  max_health }, keywords_of -> Vec<Keyword>, unit_view}` (part 2.2).
- `zones::{is_unit_token(&GameState, &CardInstance) -> bool, move_to_zone(&mut GameState, &mut CardInstance,
  OffFieldZone, MoveToZoneOptions: Default) -> MoveResult, OffFieldZone::{Hand, Graveyard, Exile},
  MoveResult::Vanished}` (part 2.1); the moved card is read back as it landed (`token.zone.z()`).
- `reduce::{reduce(&GameState, &Action), begin_game(&GameState)} -> ReduceResult { state, events, error }`,
  `reduce::legal_actions(&GameState, PlayerId) -> Vec<ActionBody>` (part 5.2).
- `setup::{mulligan_prompt_for(&GameState, PlayerId) -> Option<&PendingChoice> (or owned), deal_coins(&mut
  EngineSink)}` (part 5.3); `view_for::view_for` (part 5.2); `replay::{fold(&FoldArgs) -> FoldResult {
  state, errors }, hash_state, FoldArgs: Deserialize}` (part 5.1); `state::{create_game, find_instance,
  find_instance_mut}` (part 1).
- Effects in tests: `effects::{summon, set_radiant, fuse_cards}` with their args `Deserialize` (part 7).

## GAPS
- None of my functions is left out.
- `t_felinor`'s TS test reads `catalog.cost(cardDef(…))` from `packages/cards/src/query.ts`; that object
  method is the engine's `queryCost` re-exported, so the port calls `jackioh_engine::catalog::query_cost`
  directly and does not depend on how part 9 shapes `crate::query::catalog`.
- If `MoveResult` derives `PartialEq`/`Debug`, the `matches!(moved, MoveResult::Vanished)` checks could be
  `assert_eq!`; written with `matches!` so they compile either way.
- If `def_of` returns `Option`, the four `def_of(Some(state), id).base…` reads need an `.expect(…)`
  (TS's `defOf` throws on a missing def, so I assumed it panics and returns the def).
- `FoldArgs` (`replay::ReplayInput`) is built with `json_as` from TS's literal `{ seed, decks, handicaps,
  log }`, so it must derive `Deserialize` (the `replay` CLI reads it from stdin anyway).

## Decisions
- Ceaseless Void's `PRINTED_COST = queryCost(def)` (a TS module constant): read once in `script()`
  through `crate::card_def(ID)` and moved into the cost hook (`cost_now(state, printed_cost)`), not a
  static (SURFACE §3 allows only the catalog and the registry as statics).
- `export const radiant: Script = base` is `base.clone()` (shared `Arc`s): the Void's "radiant toBe base"
  test checks `Arc::ptr_eq` on its three hooks; the Coin's "base not toBe radiant" checks they differ.
- TS `expect(script).toEqual({})` and `Object.keys(script).sort()` are a private `set_fields(&Script)` that
  lists the TS names of every set field of part 1's `Script` (all 36), copied into each file (rule 5).
- TS `expect(x).toMatchObject({ current: 4, max: 4 })` on mana, `faceOf`/`statsWithBuffs` `toEqual` are
  field comparisons; view and event checks compare JSON (`serde_json::to_value`), as TS compared objects.
- TS `held(s).costMod = -2` wrote through the live hand card: ported as `find_instance_mut` by id.
- TS `setCounters(s, partial)` merges a JSON partial into `state.counters` and writes it back with
  `json_as` (so the four TS keys stay the literal's).
- TS `Action` literals are `Action::new(ActionBody::…, player, nonce)`; the Coin's `play` body is
  `json_as(json!({ "type": "play", … }))` so absent optionals stay absent.
- Every `#[test]` starts with `crate::register_all()` (part 5.1: the testkit cannot name the cards crate).
