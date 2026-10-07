# Slice: part 25 (engine tests 2), chunk 1 of 7
BUILDS-RUN: 0

## FILES
- `crates/engine/tests/rules/activate.rs` ← `packages/engine/test/activate.test.ts` (37 tests, 9 mods)
- `crates/engine/tests/rules/ai_policy.rs` ← `aiPolicy.test.ts` (8, 1)
- `crates/engine/tests/rules/audit.rs` ← `audit.test.ts` (5, 1)
- `crates/engine/tests/rules/board_history.rs` ← `boardHistory.test.ts` (13, 4)
- `crates/engine/tests/rules/call_to_chaos.rs` ← `callToChaos.test.ts` (26, 2)
- `crates/engine/tests/rules/call_to_chaos_plus.rs` ← `callToChaosPlus.test.ts` (19, 1)
- `crates/engine/tests/rules/combo_index.rs` ← `comboIndex.test.ts` (14, 2)
- `.fullsend/notes/spec-gaps-part-25-1.md` (four partial assertions, none dropped)

Every TS `it` is one `#[test]`, every `describe` one `mod`, in TS order, with the header comment and
every rule-stating comment kept. Ruling ids in a title lead the Rust name (SURFACE §7.3); `§x.y` becomes
`sx_y`.

## GAPS
Names these files call that other parts provide (TS name snake_cased at its TS module's Rust path,
SURFACE §4). Signatures are this chunk's guesses where SURFACE and part 1 fix none.

Fixtures (part 24, `crates/engine/tests/rules/fixtures/`), reached as `crate::rules::fixtures::<file>`:
- `harness`: `new_game(&str) -> GameState`, `setup_catalog()`, `put(&mut GameState, &str, ZoneSlot) ->
  CardInstance`, `slot(PlayerId, Row, i32) -> ZoneSlot`, `in_hand(&mut GameState, &str, PlayerId, count) ->
  Vec<CardInstance>`, `set_library(&mut GameState, PlayerId, &[&str]) -> Vec<CardInstance>`,
  `events_of_type(&[GameEvent], GameEventType) -> Vec<&GameEvent>` (results are read through serde_json,
  so `Vec<GameEvent>` works too). No `sink_for` is called: see Decisions.
- `catalog::vanilla_deck(i32, i32) -> Vec<String>`; `combat::{big_body(), plain()} -> CardDef`.
- `field::{playing(&str) -> GameState, act(&GameState, ActionInput) -> GameState, spatula(), tower(),
  watcher(), banner()}`.
- `board_history::{phoenix(), rewind(), register_board_history_fixtures()}`.
- `activate::{activate_scripts() -> IndexMap<String, CardScripts>, activate_catalog(CardDefs) -> CardDefs,
  notes(&GameState) -> Vec<String>}`, its defs as zero-arg fns (`pinger()`, `sentry()`, `high_teller()`, …,
  `log_card()`), and consts `LOG_LANE`, `MERCHANT_PRICE`, `ZAPPER_DAMAGE`, `LOW_TELL`, `HIGH_TELL: i32`,
  `PICK_KEY`, `SCEPTER_KEY`, `KEEPER_KEY: &str`.
- `call_to_chaos_plus::{chaos_plus_catalog(CardDefs) -> CardDefs}` and its defs as zero-arg fns
  (`plus()`, `core95()`, `golem()`, `fruit()`, `grape()`, `book()`, `book_token()`, `classic()`,
  `immutable()`, `trap()`, `hard_field()`).

Engine (parts 2–8), by module:
- `zones`: `place_on_field(&mut GameState, CardInstance, ZoneSlot, PlaceOnFieldOptions { stack:
  Option<bool> }) -> bool` (+ `Default`), `move_to_zone(&mut GameState, &str, OffFieldZone,
  MoveToZoneOptions) -> MoveResult`, `OffFieldZone { Hand, Library, Graveyard, Exile }`,
  `remove_from_any_zone(&mut GameState, &str)`, `fresh_face_down_id(&mut GameState, &mut CardInstance) ->
  String`, `card_at(&GameState, ZoneSlot) -> Option<&CardInstance>`, `zone_contents(&GameState, ZoneSlot) ->
  Vec<CardInstance>`, `is_locked`, `lock_zone`, `reserve_zone(&mut GameState, ZoneSlot)`,
  `reserve_home(&mut GameState, ZoneSlot, &str)`.
- `resolve`: `make_context(EngineSink, Option<CardInstance>, HookOptions) -> EffectContext`, `HookOptions:
  Default { controller: Option<PlayerId>, targets, modes, data }`, `apply_effects(&[Effect], &mut
  EffectContext)`, `cast_card(&mut EngineSink, CardInstance, CastOptions)`, `CastOptions: Default`.
- `catalog::{registered_catalog, def_of(&GameState, &str) -> &CardDef, query_cost(&CardDef) -> i32}`,
  `scripts::registered_scripts()` (cloned into an `IndexMap<String, CardScripts>`).
- `prompts::open_prompt(&mut EngineSink, OpenPromptArgs { player, kind, aim, prompt, options, min, max,
  budget, owner, resume })`.
- `replay::fold(&FoldArgs) -> FoldResult`, `FoldArgs: Default { seed, decks, log, .. }`, `FoldResult {
  state, errors }`.
- `work::{remember_on(&mut IndexMap<String, Value>, &IndexMap<String, Value>, &str, Value),
  script_step_for(&Script, &Resume) -> Option<Hook>}`; `state_check::{state_check(&mut EngineSink),
  DEATHS_WORK}`; `turn::end_turn(&mut EngineSink)`; `layers::unit_view(&GameState, &CardInstance) ->
  UnitView { attack, max_health, .. }`; `mana::effective_cost(&GameState, &CardInstance) -> i32`;
  `view_for::HIDDEN_ID`; `effects::{choose_mode(ChooseModeArgs: Deserialize), DELAYED_DESTROY_HOOK}`.
- `subsystems::audit::{AuditTargetsArgs { controller, active, loc, more, enemy_only }, audit_targets(&GameState,
  AuditTargetsArgs) -> Vec<CardInstance>, lines_of_code(&GameState, &str) -> i32}`.
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs: Default {
  ingredients: Vec<CardInstance>, target: Option<CardInstance>, .. }}`.
- `subsystems::ai_policy::{choose_action(&GameState, PlayerId, &mut Rng, PolicyOptions) ->
  Option<ActionBody>, policy_actions(&GameState, PlayerId, PolicyOptions) -> Vec<ActionBody>, PolicyOptions:
  Default { skip: Option<Vec<ActionType>> }, play_out_turn(&mut EngineSink, PlayerId, PolicyOptions) ->
  PlayoutResult { actions: Vec<Action>, stopped: PlayoutStop, error: Option<String> }, PlayoutStop
  (PartialEq, Debug), AI_SKIPPED_ACTIONS: &[ActionType]}`; `AI_PLAYOUT_STEP_CAP` read from config.
- `subsystems::board_history::{record_board_snapshot(&mut GameState), snapshot_for(&GameState, i32) ->
  Option<&BoardSnapshot>, restore_board(&mut EngineSink, PlayerId, i32, &[PlayerId]) -> Option<i32>}`.
- `subsystems::combo_index::{Grade (E, D, C, B, A, S; Copy, Serialize as the letter), GRADES: &[Grade],
  grade_name(i32) -> Grade, grade_value(Grade) -> i32, grade_of, grade_name_of(&CardInstance),
  is_terminal_grade, plays_this_turn, played_cards_this_turn(&GameState, PlayerId) -> Vec<CardInstance>,
  grade_rises(&GameState, &CardInstance), start_grade(StartGradeArgs), raise_grade(RaiseGradeArgs) (both
  Default), step_e … step_a, grade_step_effects, cascade_effects, combo_index_end_of_turn(&mut EngineSink,
  &CardInstance) -> Vec<Effect>}`.
- `subsystems::call_to_chaos::{ChaosEffectDef { name, label, build: fn() -> Effect }, CHAOS_EFFECTS:
  &[ChaosEffectDef], CHAOS_RECURSION: &str, CHAOS_CHAIN_KEY: &str, roll_chaos_effects(&mut Rng, bool,
  &[ChaosEffectDef]) -> Vec<_>, call_to_chaos(CallToChaosArgs: Default { radiant: Option<bool>, table:
  Option<&'static [ChaosEffectDef]> }), chaos_chain_of(Option<&CardInstance>) -> i32,
  chaos_chain_cap_reached(i32) -> bool, the ten effect constructors}`;
  `subsystems::call_to_chaos_plus::CHAOS_PLUS_EFFECTS: &[ChaosEffectDef]`.
- `subsystems::activate::{why_cannot_activate_ability(&GameState, PlayerId, &str, Option<&str>) ->
  Result<(), EngineError>, activate_ability(&mut EngineSink, PlayerId, &ActivateAction) -> Result<(),
  EngineError>, ActivateAction = ActionBody, activate_actions_for(&GameState, PlayerId, &CardInstance) ->
  Vec<ActionBody>, abilities_of(&GameState, &CardInstance) -> Vec<ActivationDecl>, uses_allowed(&CardInstance,
  &ActivationDecl) -> i32, is_acting_on_field(&GameState, &CardInstance) -> bool, ACTIVATION_WORK}`;
  `subsystems::hero_power::POWER_KEY`.
- testkit (part 5): `register_catalog(CardDefs)`, `register_scripts(IndexMap<String, CardScripts>)`.

## Decisions
- **`sinkFor` is not called.** A Rust fn cannot return an `EngineSink` that borrows an rng it made, so
  every TS `sinkFor(state, events)` is `EngineSink::new(&mut state, &mut events, &mut rng)` over
  `rng = Rng::new(&state.seed, state.rng_cursor)` (exactly what `sinkFor` did), via small local helpers
  (`sink_events`, `restore`, `fuse_onto`). TS's aliasing of `sink.state` and `state` becomes reads through
  `sink.state` while the sink lives, or a sink scoped in a block; no ordering of a draw changed.
- **Live objects.** TS tests held live `CardInstance`s and wrote through them; the ports hold the clones the
  helpers return, read current values with `find_instance` and write with `find_instance_mut` (local `card`
  / `card_mut` helpers).
- **Card arguments.** An engine fn that changes a card already in the state takes its id (`move_to_zone`,
  `remove_from_any_zone`); one that places a card in no pile takes the `CardInstance` by value
  (`place_on_field`, `cast_card`); a read takes `&CardInstance`. `fresh_face_down_id` takes `&mut
  CardInstance` (TS: "called on a card that is in no pile").
- **Arguments.** TS anonymous argument objects are `<FnName>Args`/`<FnName>Options` structs with `Default`
  (`AuditTargetsArgs`, `CallToChaosArgs`, `PlaceOnFieldOptions`, …); a TS default parameter is passed
  explicitly (`CHAOS_EFFECTS`, `Default::default()`).
- **Refusals** (`string | null`) are `Result<(), EngineError>` (SURFACE §4.4.9); tests compare
  `.err().map(|e| e.message)` with TS's text verbatim. `ReduceResult.error` is `Option<String>`.
- **Fixture values.** TS `CardDef` constants are zero-arg fns; `Record<string, CardScripts>` constants are
  snake_cased zero-arg fns (`activate_scripts()`); numbers and strings stay consts. A local `def(name, …)`
  whose index came from a module counter takes the index it had, written out.
- **JSON reads.** Events and views are asserted through `serde_json` (`toEqual` → JSON equality,
  `toMatchObject` → a local `matches_object`, `not.toHaveProperty` → `lacks`), so no event variant's field
  layout is assumed. Card definitions are JSON literals through `json_as`. `events_of_type` takes a
  `GameEventType`.
- **A hook that records into the test** (callToChaos's `depths`) reports down a `std::sync::mpsc` channel:
  a hook is `Fn + Send + Sync`, and `Mutex`/`RefCell` are banned by clippy.toml.
- **TS's module-level nonce counter** (activate) is a `static AtomicU32`, unique across parallel tests.
- **Test-order dependence.** Two TS tests leaned on a catalog an earlier test registered
  (boardHistory's fold, activate's replay calls `setupCatalog()` itself); each Rust test is its own thread
  with a fresh override, so the fold test calls `setup_catalog()` first.
- `put(…, { radiant: true })` is `put` then the flag set (identical for these fixtures, which print no
  Brittle and whose type does not change with the face).
- A card put back on the field from the hand is taken out of the hand pile first (spec-gaps file).
