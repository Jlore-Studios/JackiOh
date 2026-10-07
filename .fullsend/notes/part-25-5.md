# Slice: part 25 (engine tests 2), chunk 5 of 7 (#413, parent #306)
BUILDS-RUN: 0

## FILES
- `crates/engine/tests/rules/rulings_a.rs` ← `packages/engine/test/rulings-a.test.ts` (40 tests, 1 mod:
  R1–R42 less R27/R28, which TS tests elsewhere)
- `crates/engine/tests/rules/rulings_b.rs` ← `packages/engine/test/rulings-b.test.ts` (42 tests, 1 mod:
  R43–R84)
- `.fullsend/notes/spec-gaps-part-25-5.md` (one assertion not ported, two written against the nearest
  Rust observable)

Every `describe`/`it` in TS order, the header comments and every comment that states a rule or cites a
ruling or a § kept. No `todo!`, `unimplemented!`, `#[ignore]` or `// TODO`.

## SURFACE
Matched §4 (names, types), §7.3 (test names: `it("R17 R427 …")` → `fn r17_r427_…`, the describe as
`mod spec_11_rulings_r1_r42_m3_gate`), §8 (`use jackioh_engine::testkit::*;`, the thread-local
`register_catalog`/`register_scripts` on top of `registered_catalog().clone()`/`registered_scripts().clone()`),
and part 1's frozen types where they differ from SURFACE (hooks take `&mut EffectContext`; `EngineSink`,
`EffectContext`, `TriggerDef::new(..).with_when(..)`, `read_hook`, `StaticFlags`, `SetStat`, `ModifierKind`,
`ModifierExpiry`, `DelayedAt`, `Resume`, `Exertion`, `HeroState`, `AttackHealth`, `CardCost`, `PromptOption`
from `script.rs`/`state.rs`/`wire`).

## DEPENDS-ON / GAPS
Tests not ported: none. One assertion of R79 is not ported (`spec-gaps-part-25-5.md`).

Names called that another part provides (TS name snake_cased at its TS module's path; the shape assumed):

Fixtures (part 24):
- `rules::fixtures::harness` (24.6): `new_game(&str, Option<_>) -> GameState` (`None`), `put(&mut
  GameState, &str, ZoneSlot, Value) -> CardInstance` (`json!({})`, or `json!({ "radiant": true })`; a copy),
  `slot(PlayerId, Row, i32) -> ZoneSlot`, `in_hand(&mut GameState, &str, PlayerId, i32) ->
  Vec<CardInstance>`, `set_library(&mut GameState, PlayerId, &[&str]) -> Vec<CardInstance>` (24.1 and 24.5
  assumed `&[String]`: part 31 settles one), `sink_for(&mut GameState) -> EngineSink` (24.6's one-argument
  form), `events_of_type(&[GameEvent], GameEventType) -> Vec<GameEvent or &GameEvent>` (only iterated and
  serialised), `play_random_game(&str, Option<_>) -> RandomGame { state, .. }`.
- `rules::fixtures::scripts` (24.7): `infinite_reserves() -> CardDef`.

Engine:
- `resolve`: `make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext` (part
  3.2's), `HookOptions { controller, targets, .. }: Default` (all `Option`), `apply_effects(&[Effect],
  &mut EffectContext)`, `run_hook(&mut EngineSink, &CardInstance, HookName::Cry, HookOptions)`,
  `cast_card(&mut EngineSink, &CardInstance, CastOptions: Default)`.
- `catalog`: `def_of(Option<&GameState>, &str) -> &CardDef` (part 2.2's), `query(&CatalogQueryArgs)`
  (`CatalogQueryArgs: Default + Deserialize`), `query_cost(&CardDef) -> i32`, `def_by_index(SetName, &str)
  -> Option<CardDef or &CardDef>`; `scripts::{registered_scripts(), scripts_for(&GameState, &str) ->
  CardScripts}`.
- `zones`: `card_at(&GameState, ZoneSlot) -> Option<&CardInstance>`, `active_units_of`/`dormant_units_of
  (&GameState, PlayerId)` (read through `Borrow<CardInstance>`, owned or lent), `is_unit_token`,
  `move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, <options: Default>) -> MoveResult`
  (`PartialEq + Debug`), `place_on_field(&mut GameState, &mut CardInstance, ZoneSlot, PlaceOnFieldOptions
  { stack: Some(true) })`, `lock_zone`/`reserve_zone`/`release_zone(&mut GameState, ZoneSlot)`,
  `is_open`/`is_reserved(&GameState, ZoneSlot)`, `ring_order(Row, PlayerId)`, `slots_of(PlayerId, Row)`.
  Slots passed by value (`impl Into<ZoneSlot>`, part 2.1).
- `combat`: `AttackTarget::{Unit { instance }, Hero { player }}`, `why_cannot_attack(&GameState,
  &CardInstance, &AttackTarget) -> Result<(), EngineError>`, `attack_targets`, `is_active_on_field`,
  `resolve_combat(&mut EngineSink, &CardInstance, &AttackTarget)`, `declare_attack(..) -> Result<_,
  EngineError>`, `switch_position(&mut EngineSink, &CardInstance, SwitchPositionOptions: Default) ->
  Result<_, EngineError>`, `force_attack(&mut EngineSink, &CardInstance, &AttackTarget)`,
  `force_attacks_on(&mut EngineSink, &[CardInstance], &AttackTarget, Option<u32>)`, `has_exertion(&GameState,
  &CardInstance, ExertionKind::Switch)`.
- `damage`: `deal_damage(&mut EngineSink, DamageArgs { source, target, amount, flags: None }) -> i32`,
  `DamageTarget::{Unit { instance }, Hero { player }}`, `lose_health(&mut EngineSink, PlayerId, i32) -> i32`,
  `heal_hero(&mut EngineSink, PlayerId, i32)`.
- `draw` (part 4.2's shapes): `draw(&mut EngineSink, PlayerId, i32) -> Vec<DrawOutcome>`, `draw_one(&mut
  EngineSink, PlayerId, Option<ChainLinkOrCount>) -> DrawOutcome` (`ChainLinkOrCount::from(0)` for TS's
  `0`), `DrawOutcome::{Fatigue, Burned, Drawn, Cast}`, `add_to_hand(&mut EngineSink, &mut CardInstance)`,
  `shuffle_into_library(&mut EngineSink, &mut CardInstance, bool, None) -> ShuffleInOutcome::Dropped`.
- `layers`: `unit_view(&GameState, &CardInstance) -> UnitView { attack, max_health, health, keywords }`,
  `unit_has(&GameState, &CardInstance, KeywordKind)`, `stats_with_buffs(..) -> { attack, max_health }`.
- `modifiers` (part 3.2's shapes): `add_modifier(&mut EngineSink, PlayerId, ModifierExpiry, ModifierKind)
  -> PlayerModifier`, `expire_modifiers(&mut EngineSink, PlayerId)`, `schedule_delayed(&mut EngineSink,
  PlayerId, DelayedAt, Resume, None, None) -> DelayedEffect` (TS's optional `watch`, `notBefore`),
  `due_delayed(&GameState, Phase, PlayerId) -> Vec<DelayedEffect>`.
- `mana` (part 4.1's): `effective_cost(&GameState, &CardInstance, CostOptions: Default) -> i32`,
  `is_x_cost`, `printed_cost`.
- `prompts`: `open_prompt(&mut EngineSink, OpenPromptArgs { player, kind, aim, prompt, options, min, max,
  budget, owner, resume })` as a struct literal (`prompt: String`); 24.6 built it with `json_as` instead.
- `state_check::state_check(&mut EngineSink)`; `triggers::{settle(&mut EngineSink, SettleOptions:
  Default), cards_in_trigger_order(&GameState) -> Vec<TriggerHolder { card: CardInstance, .. }>}`;
  `traps::{consume_trap(&mut EngineSink, &CardInstance), is_trap_type(&GameState, &CardInstance)}`;
  `turn::{offer_draw, answer_draw(&mut EngineSink, PlayerId, bool), can_offer_draw(&GameState, PlayerId)}`;
  `view_for::{view_for, view_for_with_clock(&GameState, PlayerId, i32)}` (part 5.2's name for TS's third
  `clockMs` argument); `setup::opening_draw_for(PlayerId) -> i32`; `times_played::times_played_of(&CardInstance)
  -> i32`; `rng::create_rng(&str, u32)` (part 1's).
- `effects` (each argument built with `json_as(json!(<TS literal>))`, so only its serde shape matters):
  `damage, buff, summon, recruit, transform, steal, heal, switch_position_of, grant_keyword,
  grant_random_keywords, set_radiant, set_radiant_random, vanilla, exile, discard, discard_random,
  add_to_hand, shuffle_into, remember, choose_from_hand, destroy, discover_from_catalog,
  discover_from_graveyard, fill_board, shuffle_copies_of_self`; `targets_in_scope(&EffectContext,
  Option<&TargetScope>) -> Vec<Selection>` (part 6.1's "trailing default as `Option`"); `chosen_options(
  &EffectContext) -> Vec<String>`.
- `subsystems`: `rotation::{ROTATION_ROWS, rotate_rings(&mut EngineSink, RotationArgs: Deserialize) ->
  RotationResult { crossed, bounced }}`; `fuse::fuse(&mut EngineSink, FuseArgs: Deserialize) ->
  Option<CardInstance>` (ingredients serialised into the JSON); `scorer::{ZEPHYRS_INDEX: &str,
  candidate_defs(), rank/top_three(&GameState, PlayerId, &ScorerOptions) -> Vec<Scored { def, .. }>,
  ScorerOptions: Default}`; `activate::{activate_ability(&mut EngineSink, PlayerId, &<action: Deserialize,
  built from TS's literal>) -> Result<_, EngineError>, why_cannot_activate_ability(&GameState, PlayerId, &str,
  None) -> Result<(), EngineError>}`; `hero_power::{ensure_power(&mut EngineSink, &mut CardInstance) ->
  Option<HeroPower>, power_abilities(bool) -> Vec<ActivationDecl>, power_of(&CardInstance) ->
  Option<HeroPower { name: Serialize, x: i32 }>}`; `ai_policy` via the barrel: `AI_SKIPPED_ACTIONS`,
  `PolicyOptions { skip: Some(vec![]) }: Default` (field type a guess), `policy_actions(&GameState, PlayerId,
  PolicyOptions)`, `choose_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>`; `lethal` via the
  barrel: `projected_damage`, `is_lethal(&GameState, &CardInstance, &AttackTarget)`.

## Decisions
- Live objects: TS held and wrote through the live `CardInstance`. Every helper hands back a copy; a TS
  write (`unit.damage = 2`) is a local `edit(state, &card, |c| …)` through `find_instance_mut`, and a read
  after any change re-reads with `instance_in`. Where TS handed the engine its live card to write
  (`ensurePower(sink, card)`, R43) the test passes a copy and writes it back (`put_back`). A card handed to
  `move_to_zone`/`fuse`/`resolve_combat` is the copy read just before.
- Sinks: the harness's `sink_for(&mut state)`; while the sink is alive the state is read and written as
  `sink.state` (or `ctx.sink.state` while a context is), as TS read the state object the sink shared. TS's
  `run` and `killAndCheck`, which wrote the cursor back, build their sink with `EngineSink::new` over a local
  `Rng::new(&state.seed, state.rng_cursor)` and write `rng.cursor()` back.
- Fixture defs are built in TS's declaration order by local `ra_unit`/`ra_card` (`unit_def_of`/`card_def_of`)
  from the TS object literal as JSON with TS's spread for the overrides, so each index is the one TS's
  counter gave it; ids are `const`s, `def(id)` looks one up; `DEFS`' order is kept for registration.
- Assertions: TS `toEqual` on an object literal compares JSON (`to_json(x) == json!(…)`), so absent
  optionals read as TS's `undefined`; `toMatchObject` is a local `matches_object` with vitest's semantics
  (subset on objects, element-wise and same length on arrays); `findIndex` is `find_index` answering -1;
  `new Set` comparisons are `BTreeSet`s; a refusal `string | null` (or `{ error? }`) is `refusal(Result)`
  → `Option<String>`. Events are read as their wire JSON (`of_type`) so fields read by TS's names.
- TS default arguments are passed explicitly (`None`, `Default::default()`, `json!({})`, `in_hand(…, 1)`,
  `hand_card(…, P1)`).
- rulings-b's module `let nonce` is a `thread_local!` `Cell` (24.6 did the same).
- R14, R45: the ring and the player map read through `Display`/serialised keys; R73's two swapped
  libraries are `std::mem::swap`.

## Assumptions
`part-25-5.assumptions`: SURFACE §16's values, unchanged.
