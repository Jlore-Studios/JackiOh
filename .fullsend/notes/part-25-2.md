# Slice: part 25 (engine tests 2), chunk 2 of 7 (issue #413, parent #306)
BUILDS-RUN: 0

## FILES
Full ports, every `describe` (a `mod`) and `it` (a `#[test]`) in TS order, with the TS header and every
comment that states a rule or cites a ruling; R-ids lead the test names (SURFACE §7.3):
- `crates/engine/tests/rules/copied_text.rs` ← `copied-text.test.ts` (20 tests)
- `crates/engine/tests/rules/core_patches.rs` ← `corePatches.test.ts` (15)
- `crates/engine/tests/rules/death_pause.rs` ← `death-pause.test.ts` (8; its fixtures are local fns)
- `crates/engine/tests/rules/delayed_kinds.rs` ← `delayed-kinds.test.ts` (11)
- `crates/engine/tests/rules/fuse_registry.rs` ← `fuse-registry.test.ts` (5; local fixtures)
- `crates/engine/tests/rules/fuse_variants.rs` ← `fuse-variants.test.ts` (17)
- `crates/engine/tests/rules/fuse.rs` ← `fuse.test.ts` (18; local fixtures)
- `crates/engine/tests/rules/glitch.rs` ← `glitch.test.ts` (15; the file is 254 lines now, R764's two tests included)
Spec gaps: `.fullsend/notes/spec-gaps-part-25-2.md`. Files left: none.

## GAPS
Tests not ported: none. Assertions restated because SURFACE makes them inexpressible as written: see
the spec-gaps file (fused scripts never registered, no digest table, no live gone ingredient, glitch's
bare-object arguments).

Names called that other parts provide (module path, shape assumed; part 31 reconciles):
- Fixtures (part 24), TS names snake-cased: `fixtures::harness::{new_game(&str, Option<(Vec<String>,
  Vec<String>)>) -> GameState, put(&mut GameState, &str, ZoneSlot) -> CardInstance, in_hand(&mut
  GameState, &str, PlayerId, count) -> Vec<CardInstance>, set_library(&mut GameState, PlayerId,
  &[String]) -> Vec<CardInstance>, slot(PlayerId, Row, i32) -> ZoneSlot, events_of_type(&[GameEvent],
  GameEventType) -> Vec<&GameEvent>, setup_catalog()}`; `fixtures::catalog::{vanilla_deck(i32, i32) ->
  Vec<String>, spell_def(i32, Value /* Partial<CardDef> */) -> CardDef}`; `fixtures::combat::{plain,
  big_body}() -> CardDef`; `fixtures::copied_text::{ct() -> Ct /* fields echo, bolt, ping, asker,
  x_bolt, modal, echo_spell, draw_cast, field, caster, glow, counter, bigger, dummy: CardDef */,
  with_copied_text(GameState) -> GameState}`; `fixtures::core_patches::{core_patch_catalog() ->
  CardDefs, core_patch_scripts() -> IndexMap<String, CardScripts>, counted, uncounted, marker,
  quiet_trap: fn() -> CardDef, test_mark() -> impl Serialize /* {mark, color} */}`;
  `fixtures::turn::{LOG_LANE: i32, log_card, doom, doom_all, hurrah, later, contract, contract_ask,
  reminder: fn() -> CardDef, notes(&GameState) -> Vec<String>, turn_catalog(CardDefs) -> CardDefs,
  turn_scripts() -> IndexMap<String, CardScripts>}`; `fixtures::generation::{Run { start: String, log:
  Vec<Action>, state: GameState } (pub fields), playing(&str) -> Run, frozen(&Run) -> Run, act(&Run,
  ActionInput) -> Run, answer(&Run, Selection, Option<PlayerId>) -> Run, replayed(&Run) -> GameState,
  hand_card(&mut GameState, &str, PlayerId) -> CardInstance, pick(&str) -> Selection, gen_scripts() ->
  IndexMap<String, CardScripts>, lab_pool() -> Vec<String>, and fns -> CardDef: slime, body, fuse_a,
  fuse_b, x_unit, big_unit, immutable, field_trap, plain_trap, lab, slop, ai_unit, ai_spell,
  deck_fusion, mutate, fuser, felinor_a, felinor_b, felinor_c}`.
- Part 2: `catalog::{def_of(Option<&GameState>, &str) -> &CardDef, find_def(Option<&GameState>, &str)
  -> Option<&CardDef>, fused_id_parts / fused_id_specs / self_def_ids(Option<&GameState>, &str),
  is_digest_id(&str) -> bool, excluding_def_id(Option<&GameState>, &CatalogQueryArgs, Option<&str>) ->
  CatalogQueryArgs (Serialize, key "excludeDefId"), query(&CatalogQueryArgs) -> Vec<&CardDef>,
  pick_generated(&mut Rng, &[&CardDef], Option<&GameState>) -> Option<&CardDef>, CatalogQueryArgs:
  Default + Deserialize}`; `scripts::{script_of(&GameState, &str) -> CardScripts, registered_scripts()
  -> IndexMap<String, CardScripts>}`; `layers::unit_view(..)` with `attack`, `max_health`, `keywords`;
  `marks::marks_on(&GameState, &str)`; `own_library::own_library_view(&GameState, PlayerId)`
  (Serialize); `query::{last_spell_played(&GameState), left_field_since_resolved(&GameState,
  &GameEvent) -> bool}`; `stays::exit_mark(&GameState) -> u32`; `times_played::times_played_of(&CardInstance)
  -> i32`; `plague::plague_multiplier_of(&GameState, &CardInstance) -> i32`; `zones::{card_at(&GameState,
  impl Into<ZoneSlot>) -> Option<&CardInstance>, slots_of(PlayerId, Row) -> Vec<ZoneSlot>,
  move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, MoveToZoneOptions), place_on_field(&mut
  GameState, &mut CardInstance, ZoneSlot, PlaceOnFieldOptions) -> bool, OffFieldZone::{Hand,
  Graveyard}}`.
- Part 3: `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) ->
  EffectContext, HookOptions { controller: Option<PlayerId>, .. }: Default, cast_card(&mut EngineSink,
  &CardInstance, CastOptions), CastOptions: Default, apply_effects(&[Effect], &mut EffectContext),
  run_hook(&mut EngineSink, &CardInstance, HookName, HookOptions), HookName::{Cry, Death}}`;
  `prompts::{open_prompt(&mut EngineSink, OpenPromptArgs { player, kind, aim, prompt, options, min,
  max, budget, owner, resume }), resume_self(&EffectContext, &str, IndexMap<String, Value>) -> Resume}`;
  `state_check::{state_check(&mut EngineSink), DEATHS_WORK: &str, owed_deaths_of(&Resume) ->
  Option<DeathPass { owed: Vec<CardInstance>, .. }>}`; `work::owed_work(&GameState, Option<&str>) ->
  Vec<WorkItem>` (or `Vec<&WorkItem>`); `triggers::{settle(&mut EngineSink, SettleOptions),
  SettleOptions: Default}`; `traps::fire_traps_for(&mut EngineSink, &GameEvent)`.
- Part 4: `draw::draw(&mut EngineSink, PlayerId, i32)`; `mana::{effective_cost(&GameState,
  &CardInstance, <Default options>) -> i32, printed_cost, play_cost(&GameState, &CardInstance) -> i32}`;
  `play_steps::{PLAY_WORK_KIND: &str, run_of(&Resume) -> Option<PlayRun>}` (`PlayRun: Serialize`,
  keys `instanceId`, `cast`).
- Part 5: `reduce::{begin_game, reduce, legal_actions}`, `ReduceResult { state, events, error }`;
  `replay::{fold(&FoldArgs) -> FoldResult { state, errors: Vec<_> }, hash_state}` with `FoldArgs:
  Deserialize`; `view_for::{view_for, HIDDEN_ID: &str}`; `turn::START_OF_TURN_WORK: &str`;
  `testkit::{register_catalog(CardDefs), register_scripts(IndexMap<String, CardScripts>)}`.
- Parts 6/7 (effects, built from TS literals with `json_as`): `effects::{damage, discard_hand_at_turn_end,
  fuse_cards, fuse_generated, fuse_random_into, glitch()}`, `effects::delay::{DELAYED_DESTROY_HOOK,
  DELAYED_DISCARD_HAND_HOOK: &str}`.
- Part 8: `subsystems::copied_text::{COPIED_TEXT_KEY: &str, copied_text_of(&GameState, &CardInstance) ->
  Option<PlayRecord>, running_script_of(&GameState, &CardInstance) -> Script, text_face_of(&GameState,
  &CardInstance) -> CardInstance}`; `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) ->
  Option<CardInstance>, FuseArgs: Deserialize, fused_digest(&str) -> String, fused_ingredients(&GameState,
  &str) -> Option<Vec<String>>}`; `subsystems::glitch::{count_system_play(&mut GameState,
  &CardInstance), seat_played_by, seats_swapped}`.

## Decisions
- **Fixture names.** A TS `export const x: CardDef` is `fn x() -> CardDef` (as the other porters
  call them). A TS constant object holding defs, closures or computed strings (`CT`, `TURN_SCRIPTS`,
  `CORE_PATCH_SCRIPTS`, `GEN_SCRIPTS`, `LAB_POOL`, `TEST_MARK`) is a snake-cased fn (`ct()`,
  `turn_scripts()`, …), as part 2.1 did for `BOOK_SWAP_TRIGGER`; `CT`'s members are snake-cased fields.
  A numeric constant (`LOG_LANE`) keeps its name.
- **TS optional parameters.** A parameter with a default is passed explicitly (`in_hand(.., 1)`,
  `hand_card(.., P1)`, `resume_self(.., IndexMap::new())`, options as `Default::default()`); one without
  a default is an `Option` (`new_game(seed, None)`, `answer(run, selection, None)`,
  `owed_work(state, Some(kind))`). TS `put(.., { radiant: true })` is the three-argument `put` (the
  other porters' form) and the live card made Radiant after.
- **Sinks.** TS's `sinkFor(state)` is a local `Sink { events, rng }` whose `on(&mut state)` builds part
  1's `EngineSink::new(state, &mut events, &mut rng)` per call, so one sink's rng and events run on
  across calls as TS's shared object did while the test still edits the state between them. The
  harness's `sink_for` is not called: a returned `EngineSink` cannot own its events and rng.
- **Live objects.** Where TS read or wrote an instance after an engine call changed it, Rust re-reads it
  by id (`find_instance`) and writes through `find_instance_mut`.
- **JSON where the Rust type is not frozen.** Views, events, `PlayRecord`s, `run_of`, library lists,
  marks, a def's `cost`/`type` and similar are compared as `serde_json::to_value` against the TS
  literal: §5.1 fixes that JSON, not the Rust types part 1 did not freeze. `toMatchObject` is a local
  `matches_object`; `toMatch(/text/)` is `contains`; the id regexes are small hand checks.
- **Nonces.** TS's module-level counter is a per-file `static AtomicU32` (unique across test threads).
- **fuse_registry's `seen`.** TS's module array is a `thread_local!` `Cell<Vec<String>>` (each
  `#[test]` has its own thread; `Cell` is not in clippy.toml's list, `RefCell` is).
- **Signatures where an owner changed TS's arity** follow the owner's notes (parts 2.2, 3.2, 8.1):
  catalog readers take `Option<&GameState>`, `fused_ingredients` takes the state (its readable-id test
  passes a fresh game's), `pick_generated`'s odds and `seat_played_by`'s argument are states.
