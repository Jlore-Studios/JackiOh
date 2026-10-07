# Slice: part 26, chunk 6 of 7 (engine tests 3: view, turn, setup, combat), #414 under #306
BUILDS-RUN: 0

## FILES
All nine ported whole (every `describe` a `mod`, every `it` a `#[test]`, TS order, header and rule comments
kept); `#[test]` counts equal the TS `it` counts file by file (86 in all):
`crates/engine/tests/rules/{targeting (23), temporary (5), transform_variants (7), tribute_zones (7),
tribute (12), turn_cap (2), turn_wiring (8), turn (16), view_marks (6)}.rs`. Notes: this file,
`part-26-6.assumptions`, `spec-gaps-part-26-6.md`. Nothing outside these paths was written; no rebase
conflicted.

## GAPS
Tests not ported exactly: see `spec-gaps-part-26-6.md` (turn_wiring's `vi.mock` seam, one live-object read,
one identity check, one `beforeAll`/`afterAll` pair). None dropped.

Names called that other parts provide (TS name snake_cased at its TS module's Rust path; the shape assumed):

**Fixtures, part 24** (`crate::rules::fixtures::<file>`; shapes as part 24.6's notes give them, guessed by
the same rules for chunk 7's files):
- `harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`, `setup_catalog()`,
  `put(&mut GameState, &str, ZoneSlot, Value /* { radiant? } */) -> CardInstance`, `slot(PlayerId, Row, i32)
  -> ZoneSlot`, `in_hand(&mut GameState, &str, PlayerId, i32) -> Vec<CardInstance>`, `set_library(&mut
  GameState, PlayerId, &[String]) -> Vec<CardInstance>`, `sink_for(&mut GameState) -> EngineSink<'_>`,
  `events_of_type(&[GameEvent], GameEventType)` (a `Vec` of `GameEvent` or `&GameEvent`; read through serde).
- `combat`: statics `plain`, `temporary_body`. `catalog`: `vanilla_deck(i32, i32) -> Vec<String>`.
- `scripts` (chunk 7): statics `infinite_reserves`, `gravedigger`, `hinder`, `mana_well`, `shredder`, `x_bolt`.
- `generation`: statics `body`, `juhan`, `fuse_a`, `fuse_b`, `immutable`, `classic_unit`, `classic_plus_unit`,
  `classic_spell`; `playing(&str) -> Run` (`pub state: GameState`), `frozen(&Run) -> Run`, `act(&Run, impl
  Serialize) -> Run`, `replayed(&Run) -> GameState`, `hand_card(&mut GameState, &str, PlayerId) -> CardInstance`.
- `play_pipeline_a` (chunk 7): `PA`, a `LazyLock` of a struct with one `CardDef` per TS key snake_cased (as
  24.6's `CT`): `bolt, ping, zapper, joro, ghost, immune, chooser, echo_bolt, grave_raiser, cheap_hunter,
  medic, plague_hunter, lane_hunter, field, chalice, twin`; `with_play_a(GameState) -> GameState`.
- `play_pipeline_b` (chunk 7): `DISCOVER_POOL` (read as `&*DISCOVER_POOL`: a const slice or a `LazyLock<Vec>`),
  statics `grave_trap, reborn_body, stack_body, titan, titan_two, tribute_field`; `pb_playing(&str) ->
  GameState`, `pb_act(&GameState, impl Serialize) -> GameState`, `pb_reduce(&GameState, impl Serialize) ->
  ReduceResult`, `plays_of(&GameState, &str, PlayerId)` (a `Vec` of plays that serialise as TS's).
  TS's `only` is written locally (trivial; its fixture signature is unknown).
- `turn` (chunk 7): `LOG_LANE: i32`, statics `cast_spell, clock, crumble_watcher, log_card, reminder`,
  `TURN_SCRIPTS` (`.clone()` is an `IndexMap<String, CardScripts>`), `turn_catalog(CardDefs) -> CardDefs`,
  `note(&str) -> Effect`, `notes(&GameState) -> Vec<String>`, `write(&mut GameState, &str)`.

**Testkit, part 5**: `register_catalog(CardDefs)`, `register_scripts(IndexMap<String, CardScripts>)`; and the
new seam turn_wiring asks for (`spec-gaps-part-26-6.md`): `mock_brittle_tick`, `mock_animate_at_turn_start`,
`mock_return_at_cleanup`, each `(impl Fn(&mut EngineSink<'_>, PlayerId) + Send + Sync + 'static)`.

**Engine** (through `jackioh_engine::testkit::*` unless a path is given):
- `reduce`: `begin_game`, `reduce(&GameState, &Action) -> ReduceResult { state, events, error: Option<String> }`,
  `legal_actions -> Vec<ActionBody>`. `replay`: `hash_state`, `fold(&FoldArgs) -> FoldResult { state, errors }`
  with `FoldArgs { seed: String, decks: (Vec<String>, Vec<String>), log: Vec<Action>, .. }: Default`.
- `catalog::{registered_catalog() (.clone() → CardDefs), def_of(&GameState, &str) -> &CardDef}`,
  `scripts::registered_scripts()` (`.clone()`).
- `resolve`: `make_context(EngineSink<'_>, Option<&CardInstance>, HookOptions) -> EffectContext<'_>` (called
  with `sink.reborrow()`), `HookOptions { controller: Option<PlayerId>, targets: Option<Vec<Selection>>, .. }:
  Default`, `apply_effects(&[Effect], &mut EffectContext)`.
- `play_choices`: `declared_targets(&CardInstance) -> Vec<TargetDecl>`, `legal_selections_for(&GameState,
  PlayerId, &CardInstance, &TargetDecl) -> Vec<Selection>`, `play_actions_for(&GameState, PlayerId,
  &CardInstance) -> Vec<PlayAction>`, `why_choices_refused(&GameState, PlayerId, &CardInstance, &PlayAction)
  -> Result<(), EngineError>` (`PlayAction: Serialize + Deserialize` from TS's literal, `"type": "play"`
  included), `freed_by_tribute(&GameState, &ZoneSlot, &[String]) -> bool`, `legal_zones_for(&GameState,
  PlayerId, &CardInstance, &[String]) -> Vec<ZoneChoice>`, `tribute_cost_of(&CardInstance) -> i32`,
  `tribute_value_of(&GameState, &CardInstance) -> i32`, `legal_tribute_units(&GameState, PlayerId,
  &CardInstance) -> Vec<CardInstance>`, `legal_tribute_sets(..) -> Vec<Vec<String>>`, `SHEEP_TOKEN_INDEX: &str`
  (`SHEEP_TRIBUTE_VALUE`/`RADIANT_SHEEP_TRIBUTE_VALUE` read from config, where part 1 put them).
- `targeting`: `targeting_discards_of(&GameState, &CardInstance) -> i32`, `can_pay_to_target(&GameState,
  PlayerId, &CardInstance, Option<&str>) -> bool`. `targeting_point`: `intercept_targeting(&mut EngineSink,
  InterceptArgs) -> Vec<Selection>`, `InterceptArgs { chooser, picks: Vec<Selection>, targeting: Option<_>,
  accepts: Option<_>, what: Option<RedirectKind>, source: Option<CardType> }`, `RedirectKind { Target, Attack }`,
  `why_target_answer_refused(&GameState, &PendingChoice, &[Selection]) -> Result<(), EngineError>`.
- `zones`: `card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>`, `place_on_field(&mut GameState, &mut
  CardInstance, &ZoneSlot, options /* { stack? }, Deserialize */) -> bool`, `remove_from_field(&mut GameState,
  &CardInstance, options: Default) -> bool`, `lock_zone(&mut GameState, &ZoneSlot)`, `is_reserved(&GameState,
  &ZoneSlot) -> bool`, `active_units_of(&GameState, PlayerId)` (cards or refs).
- `combat::is_sick(&GameState, &CardInstance)`, `layers::unit_view(..).attack`, `view_for::{view_for, HIDDEN_ID}`,
  `temporary::is_temporary_card(&GameState, &CardInstance)`, `preview::is_face_down(&GameState, &CardInstance)`,
  `params::step_param(&mut CardInstance, &str, i32)`.
- `modifiers`: `add_modifier(&mut EngineSink, PlayerId, <the modifier without its id, Deserialize from
  { kind, …, expiry }>)`, `schedule_delayed(&mut EngineSink, PlayerId, DelayedAt, Resume) -> DelayedEffect`.
- `prompts`: `RESUME_HOOK`, `open_prompt(&mut EngineSink, OpenPromptArgs /* Deserialize */)`,
  `resume_self(&EffectContext, &str, <data: Default>) -> Resume`. `turn::{END_OF_TURN_WORK, START_OF_TURN_WORK}:
  &str`. `work::{owed_work(&GameState, Option<&str>) -> Vec<WorkItem>, can_resume(&Resume) -> bool}`.
  `jackioh_engine::draw::draw(&mut EngineSink, PlayerId, i32)`.
- `jackioh_engine::subsystems::combo_index::GRADES` (iterable, serialising to the six letters).
- `jackioh_engine::effects::{transform_beneath(Default::default()), transform_random, summon, add_to_hand,
  discover_from_catalog, damage, sacrifice, steal}` with `Deserialize` arguments (`json_as(json!(…))`), and
  `chosen_options(&EffectContext) -> Vec<String>`.

## Decisions
- **Fixture shapes follow part 24.6** (its notes landed mid-run; I rewrote my first four files to match):
  defs are `LazyLock` statics read as `plain.id`, so no binding here shadows an imported def name; defaults
  are passed explicitly (`in_hand(.., 1)`, `vanilla_deck(DECK_SIZE, 1)`, `hand_card(.., PlayerId::P1)`);
  `put`'s options are a `Value` (`json!({})`); zone calls take `&ZoneSlot`, as 24.6's fixtures call them
  (part 26.1 passed `ZoneSlot` by value: part 31 reconciles). Fixture action helpers take `impl Serialize`,
  so bodies go in as `json!` literals.
- **Live objects.** TS held live cards and wrote through them; here a test holds owned copies, writes with
  `find_instance_mut` and re-reads with `find_instance` before handing a card to a read that follows a change.
- **Sinks.** `sink_for(&mut state)` in a scope; while it lives the state is `sink.state`; TS's
  `sinkFor(state, events)` reads `sink.events.clone()` before the scope ends. Rng cursors are written back
  exactly where TS wrote them (view_marks' steal).
- **Expectations.** Events and views are read through their serde JSON (`toEqual` → `json!` equality,
  `toMatchObject` → a local `matches_object`, `"x" in obj` → key absence), so no event variant's field layout
  is assumed; `indexOf`/`lastIndexOf` keep TS's -1. Refusal strings are `Result<(), EngineError>`; `toMatch`
  is `contains` on `Some`.
- **Local TS definitions.** Module index counters are written out (`tb-` 1551–1560, `tn-` 2401–2406, `vm-`
  3701–3704, `pbt-` 4611–4612); `Partial<CardDef>` spreads are a local `spread` over `json!`; TS's
  `String(x)` is `js_string`. Module `let nonce`s are `static AtomicU32`s.
- **turn_wiring**: `beforeEach(recordingDoubles)` is a call at the top of each test; `mockImplementation`
  installs a new double; `mock.calls` is an `mpsc` channel per stage (no `Mutex`/`RefCell`, clippy.toml).
- **Names**: R-ids anywhere in a title lead the Rust name in title order (`(R3)` after an R389 title gives
  `r389_r3_…`); a leading `§x.y` is `section_x_y_…`, an inner one `…_x_y`; `#`/punctuation dropped.
- **Exposure, recorded honestly**: one `Grep` I meant for my own edits ran over `crates/engine/tests/rules/`
  and printed four `.id.clone()` lines of other parts' files (`fixtures/activate.rs`, `fixtures/copied_text.rs`,
  `fixtures/instance_data.rs`, `mulligan_concurrent.rs`). Nothing else under `crates/` but part 1's frozen
  files was opened; I read parts 24.1, 24.2, 24.6, 25.1 and 26.1's notes.
