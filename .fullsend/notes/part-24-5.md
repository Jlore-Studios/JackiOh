# Slice: part 24, chunk 5 of 7 — engine tests 1 (effect verbs), files 43–50 (#412, parent #306)
BUILDS-RUN: 0

## FILES
`crates/engine/tests/rules/{effects_summon_copies, effects_summon, effects_summon_this, effects_swap,
effects_targets, effects_transform, effects_tune, effects_turn_end}.rs` — every TS `it` (148) and
`describe` (26) ported in TS order, one `#[test]` per `it` and one `mod` per `describe`, the header
comments and every ruling/§ comment kept (every R-id of each TS file is in its Rust file, as a name
token or in a comment). Notes: this file, `part-24-5.assumptions`, `spec-gaps-part-24-5.md`. Nothing left.

## SURFACE
Matched §4 (paths, names, types), §6.5/§6.6 (`EngineSink::new`, `Effect.apply` called as
`(effect.apply)(&mut ctx)`), §7.3 (test names), §8 (testkit: `register_catalog`, `register_scripts`,
`json_as`, `json!`). Part 1's frozen types used as written (`CardInstance`, `GameState`, `GameEvent`,
`Selection`, `Zone`, `ZoneRef`, `Exertion`, `Counters`, `AttackHealth`, `BrittleCounter`, `Tuning`,
`TuningChange`, `HeroState`, `PlayerModifier`/`ModifierKind`, `AppliedAction`, `CreateGameOptions`,
`Action`, `Phase`, `Position`, `Script`, `CardScripts`, `EffectContext`, `Rng`).

## GAPS
Tests not ported: none (`spec-gaps-part-24-5.md` lists the assertions written against the nearest Rust
observable).

Names called that another part provides (module path; the shape these files assume). The fixture
shapes follow the majority of parts 24.1–24.4's notes, so one reconcile fits all of them.

Fixtures (part 24, chunks 6–7):
- `rules::fixtures::harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`;
  `put(&mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance` (three arguments, answering a copy
  of the placed card; TS's two `{ radiant: true }` calls are a local `put_radiant`, see Decisions);
  `slot(PlayerId, Row, i32) -> ZoneSlot`; `in_hand(&mut GameState, &str, PlayerId, count) ->
  Vec<CardInstance>` (count always passed as a literal, so `i32` or `usize`); `set_library(&mut
  GameState, PlayerId, &[String]) -> Vec<CardInstance>`; `setup_catalog()`; `events_of_type(&[GameEvent],
  GameEventType) -> Vec<_>` (of `GameEvent` or `&GameEvent`: only `len`, `is_empty` and serialising are
  used; fields are read by matching `GameEvent` itself). Not called: `sink_for` (see Decisions).
- `rules::fixtures::catalog`: `unit_def(i32, Value) -> CardDef`, `spell_def(i32, Value) -> CardDef` (TS's
  `Partial<CardDef>` overrides as the JSON literal), `vanilla_deck(i32, i32) -> Vec<String>`. Not called:
  `token_def` (`tokenDef("rush")` is read only for its id, written `"fx-token-rush"`).
- Card fixtures, each TS `export const x: CardDef` as `pub fn x() -> CardDef` (called `x().id`):
  `combat::{plain, stacker, taunter}`; `prompts::{deck_asker, deck_watcher, grave_watcher, grunt,
  recurring, snare, spark, striker, wardrum}` and `prompts::quickdraw_of(&CardDef) -> CardDef`;
  `instance_data::{activator, billy, body, brittle_unit, constant, dear_body, echo_bolt, free_body,
  numbered, numbered_body, shackled, stoic, tributer, x_bolt}`; `turn::{cut_asker, cutter, log_card,
  marker, one_more, questioner, rate_limit, tempo, two_more}`.
- `rules::fixtures::prompt_harness`: `board(&str) -> GameState`; `act(&GameState, body, Option<&mut
  Vec<Action>>) -> GameState` (body built with `json_as(json!(TS literal))`, so `ActionInput` or `Value`
  both compile); `answer_keys(&mut GameState, &[&str])` and `open_as(&GameState, PromptKind, PlayerId)`
  (returns unread); `round_trip(&GameState) -> GameState`; `replayable(&str, &[String], &[String])`
  answering a struct with pub `state: GameState`, `log: Vec<Action>`, `decks`; `expect_replays(&str,
  &decks, &Vec<Action>, &GameState)`; `hand_card(&GameState, PlayerId, &str)` (owned or `&`, `.id` read).
- `rules::fixtures::instance_data::instance_game(&str) -> GameState` (TS's optional decks omitted, as
  parts 24.1 and 24.3 assume).
- `rules::fixtures::turn`: `turn_catalog(CardDefs) -> CardDefs`, `scripts() -> IndexMap<String,
  CardScripts>` (TS `TURN_SCRIPTS`, by the brief's step 2; part 25.1 assumed `activate_scripts()` for its
  file, so part 31 may want one rule), `notes(&GameState) -> Vec<String>`, `LOG_LANE: i32`.

Engine (parts 2–8), as their owners' notes give them where they do:
- `catalog::registered_catalog()` and `scripts::registered_scripts()` (either a reference or an owned
  copy: both are `.clone()`d); the testkit's `register_catalog(CardDefs)`, `register_scripts(IndexMap<
  String, CardScripts>)`; `scripts::scripts_for(&GameState, &str) -> CardScripts`.
- `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext,
  HookOptions { controller, targets, modes: Option<_>, .. }: Default, run_hook(&mut EngineSink,
  &CardInstance, HookName, HookOptions), HookName::Cry}` — part 3.2's shape (the owner's notes).
- `zones::{ZoneSlot (= wire::ZoneRef, Copy), card_at(&GameState, ZoneSlot) -> Option<&CardInstance>,
  pile_at(&GameState, ZoneSlot) -> Option<&Pile>, lock_zone, reserve_zone(&mut GameState, ZoneSlot),
  place_on_field(&mut GameState, &mut CardInstance, ZoneSlot, PlaceOnFieldOptions { stack:
  Option<bool> }: Default) -> bool, move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone,
  Default::default()), OffFieldZone::{Hand, Graveyard, Exile}, active_units_of(&GameState, PlayerId)}`.
- `layers::{unit_view(&GameState, &CardInstance) -> UnitView { attack, max_health, health, keywords }`
  (also `Serialize`, camelCase, for TS's `toMatchObject`), `unit_has(&GameState, &CardInstance,
  KeywordKind) -> bool`, `stats_with_buffs(&GameState, &CardInstance)` (`Serialize` as `{ attack,
  maxHealth }`)}`; `combat::is_sick(&GameState, &CardInstance) -> bool`.
- `triggers::{cards_in_trigger_order(&GameState) -> Vec<TriggerHolder>, trigger_holders_for_event(
  &GameState, GameEventType) -> Vec<TriggerHolder>, dispatch_event(&mut EngineSink, &GameEvent)}`,
  `TriggerHolder { card: CardInstance, zone: TriggerZone, .. }`, `TriggerZone::{Hand, Library,
  Graveyard}` (`PartialEq + Debug`).
- `subsystems::fuse::fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>`, `FuseArgs` built with
  `json_as` from TS's `{ ingredients, toHand }` / `{ ingredients, target }` (part 8.1: it derives serde).
- `reduce::{reduce, begin_game, seat_to_act(&GameState) -> Option<PlayerId>, ReduceResult}`;
  `replay::{hash_state, fold(&FoldArgs)}` with `FoldArgs` built by `json_as` from `{ seed, decks, log }`
  and the answer's `state` and `errors: Vec<_>`; `view_for::{view_for, HIDDEN_ID: &str}`;
  `modifiers::turn_ends_of(&GameState, PlayerId) -> Option<&PlayerModifier>`.
- `mana::effective_cost(&GameState, &CardInstance, CostOptions: Default) -> i32` (part 4.1);
  `numbers::numbers_on(&GameState, &CardInstance) -> Vec<NumberOnCard>` (read as JSON: `id`, `ref`,
  `label`, `value`); `params::param_value(&GameState, Option<&CardInstance>, &str, Default) -> i32`;
  `tuning::{tuned_count(&CardInstance, &str, i32), x_of(&CardInstance)} -> i32`;
  `echo::printed_echo(&CardInstance, &GameState) -> i32` (part 4.1: the state is required);
  `jackioh_engine::draw::draw(&mut EngineSink, PlayerId, i32)` (full path: the effects' `draw` is a verb).
- Effects, data arguments built with `json_as(json!(TS literal))` (TS `= {}` as `json!({})`): `summon`,
  `summon_copy`, `summon_random`, `recruit`, `fill_board`, `damage`, `swap`, `transform`, `vanilla`,
  `degrade`, `upgrade`, `set_number`, `shuffle_copies_of_self`, `shuffle_into`, `end_turn`,
  `end_turn_after_actions`; no-argument verbs `swap_health()`, `swap_board()`, `swap_library()`;
  `effects::tune::{tune_once(&mut EffectContext, &CardInstance, TuneDirection, bool), applicable_changes(
  &GameState, &CardInstance, TuneDirection) -> Vec<TuneRow>` (`TuneRow: Serialize`, compared as its
  names), `TuneDirection::{Degrade, Upgrade}}`; `effects::targets::{TargetSpec, BoardScope: Default
  (both built with json_as), resolve_target(&EffectContext, &TargetSpec) -> Option<DamageTarget>,
  instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>` (serialised to compare),
  `cards_in_scope(&EffectContext, &BoardScope)`, `adjacent_to(&EffectContext, &TargetSpec, &BoardScope)`
  (each answer read through `Borrow<CardInstance>`, owned or lent), `matches_scope(&EffectContext,
  &CardInstance, &BoardScope) -> bool}`; `damage::DamageTarget::Unit { instance, .. }`.

## Decisions
- TS's `sinkFor(state)` is built in each file from part 1's `EngineSink::new(state, &mut events, &mut
  Rng::new(&state.seed, state.rng_cursor))` (a Rust sink borrows its event list and rng, so a harness
  function cannot hand one back). Each TS `run` helper is a local `run` that builds the sink and
  `make_context`, applies the effect, writes the cursor back where TS did (`state.rngCursor =
  sink.rng.cursor`) and answers the events; TS's `ctxOf`/`makeContext(sinkFor(state), …)` reads are a
  local `with_ctx(state, self, f)` that runs the reads inside `f`; a TS sink shared by several contexts
  (`effects-turnEnd`'s two riders) is one sink with a context made per call, as TS did.
- Live objects: after any effect or action a card is read again by id (`live`), and TS's writes through
  the object go through `find_instance_mut` (`live_mut`); a card handed to `tune_once`, `move_to_zone` or
  `place_on_field` is the live copy read just before.
- TS's `put(state, id, slot, { radiant: true })` is a local `put_radiant` written from `new_instance`
  (flag set before placing, TS's order) and `zones::place_on_field`, so every harness `put` call has
  three arguments; `set_library`'s ids go through a local `owned(&[&str]) -> Vec<String>`.
- TS `toEqual` on object literals compares JSON (`serde_json::to_value` of the Rust value against
  `json!(TS literal)`), so absent optionals match TS's; `toMatchObject`/`objectContaining` is a local
  `matches_object` (subset on objects, element-wise on arrays); `toContainEqual` is `contains` on the
  frozen type or on JSON. Sets (`new Set(...)`) are `IndexSet`s, compared as sets.
- TS `indexOf`/`lastIndexOf` comparisons keep TS's `-1` for a missing entry (`index_of` answers `i64`).
- Test names by §7.3: `R<n>` → `r<n>` tokens where the title has them (a possessive `R68's` is the token
  `r68`), `§10.7` → `s10_7`, `#61` → `61`, `E26` → `e26`.
- TS module counters (`let nextIndex = 760`, raised by each `defOfKind`) are the indexes they produced,
  written at each def; `defOfKind`'s `...overrides` is a JSON spread before `json_as::<CardDef>`.
- Fixture defs are zero-argument functions (each test builds what it reads); `{ type: "Field Spell" }` on a
  `spellDef` result sets `type_`.
- `effects-turnEnd`'s module `let nonce` is a `thread_local!` `Cell<u32>`; its `step` (a closure over the
  game and log in TS) is a nested `fn` taking both.
