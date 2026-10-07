# Slice: part 25 (engine tests 2: rulings, subsystems, prompts and triggers), chunk 7 of 7 (#413)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run. No Rust under
`crates/*/src/` other than part 1's frozen files was read, and no other part's test or fixture file.

## FILES
`crates/engine/tests/rules/{statecheck, stays, trap_cardresolved, trap_window_pause, trigger_zones,
triggers, twice_forward}.rs` — every TS `it` (62) and `describe` (12) ported in TS order, header
comments and every ruling/§ comment kept, each TS file's local helpers as private fns. Notes: this
file, `part-25-7.assumptions`, `spec-gaps-part-25-7.md`. Nothing left.

## GAPS
Tests not ported: none (see `spec-gaps-part-25-7.md` for the assertions written against the nearest
Rust observable).

Names called that another part provides (module path, the shape these files assume):

Fixtures (part 24; the same shapes part 24.1's notes assume):
- `crate::rules::fixtures::harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`;
  `put(&mut GameState, &str, ZoneSlot) -> CardInstance`; `slot(PlayerId, Row, i32) -> ZoneSlot`;
  `in_hand(&mut GameState, &str, PlayerId, count) -> Vec<CardInstance>`; `set_library(&mut GameState,
  PlayerId, &[String]) -> Vec<CardInstance>`; `events_of_type(&[GameEvent], GameEventType)` returning a
  `Vec` of the events (`&GameEvent` or `GameEvent`: only `.len()`, `.is_empty()`, `.iter()` with a
  `GameEvent` match and `serde_json::to_value` are used).
- `fixtures::catalog::token_def(&str) -> CardDef`; `fixtures::scripts::double_edge() -> CardDef`.
- `fixtures::twice_forward::{forward, spell, self_exiler, trap, turner, crier, caster}() -> CardDef`,
  `CRIER_DAMAGE`, `TURNER_DAMAGE: i32`, `twice_forward_catalog(CardDefs) -> CardDefs`, and TS's
  `TWICE_FORWARD_SCRIPTS` as `twice_forward_scripts()` returning an iterable of `(String, CardScripts)`
  (an `IndexMap<String, CardScripts>`): a map of closures cannot be a `const`.

Engine (parts 2–8):
- `stays::{left_field_since(&[GameEvent], usize, &str) -> bool, moves_in(&[GameEvent] (or impl
  IntoIterator<Item = &GameEvent>), Option<&GameState>) -> LaterMoves { moved: IndexSet<String>,
  controller_before: IndexMap<String, PlayerId> }, note_uncovered(&mut GameState, &str, Option<&str>),
  note_moved(&mut GameState, &str), note_reported(&mut GameState, &GameEvent), uncovered_by(&GameState,
  &GameEvent) -> Vec<String>}`.
- `state_check::state_check(&mut EngineSink)`, panicking with TS's text "… did not settle in 100
  passes …" at `config::STATE_CHECK_PASS_CAP` (`usize`).
- `resolve::{run_hook(&mut EngineSink, &CardInstance, HookName, HookOptions), make_context(EngineSink,
  Option<CardInstance>, HookOptions) -> EffectContext, apply_effects(&[Effect], &mut EffectContext),
  cast_card(&mut EngineSink, &CardInstance, CastOptions)}`; `HookName` the enum of TS's string union
  (`Cry`, `StartOfTurn`, `EndOfTurn`, `OnPlayHook`, …); `HookOptions { controller: Option<PlayerId>, … }`
  and `CastOptions` both `Default`.
- `damage::{deal_damage(&mut EngineSink, DamageArgs) -> i32, DamageArgs { source: Option<CardInstance>,
  target, amount, flags: Option<_> }, DamageTarget::Unit { instance: CardInstance }}`.
- `layers::{unit_view(&GameState, &CardInstance) -> UnitView { attack, max_health, health, position, .. },
  unit_has(&GameState, &CardInstance, KeywordKind) -> bool}`.
- `zones::{card_at(&GameState, ZoneSlot) -> Option<&CardInstance>, first_free_zone(&GameState, PlayerId,
  Row) -> Option<ZoneSlot>, is_locked(&GameState, ZoneSlot) -> bool, lock_zone(&mut GameState, ZoneSlot),
  place_on_field(&mut GameState, &mut CardInstance, ZoneSlot, options) -> bool, active_units_of(&GameState,
  PlayerId) -> Vec<&CardInstance>}`; the options bag `Default + Deserialize` (`{ "stack": true }`).
- `traps::{traps_watching(&GameState, &GameEvent) -> Vec<TrapMatch { trap: CardInstance, .. }>,
  fire_traps_for(&mut EngineSink, &GameEvent) -> TrapDispatch { fired: Vec<String>, paused: bool },
  run_trap_window(&mut EngineSink, &GameEvent) -> TrapDispatch, is_trap_window_event(&GameEvent) -> bool,
  TRAP_WINDOW_EVENTS: &[GameEventType], TRAP_WINDOW_WORK: &str, owed_window_of(&Resume) ->
  Option<OwedWindow { event: GameEvent, owed: Vec<String>, .. }>}`.
- `triggers::{settle(&mut EngineSink, SettleOptions), cards_in_trigger_order(&GameState) ->
  Vec<TriggerHolder>, trigger_holders_with_hook(&GameState, HookName, Option<PlayerId>) ->
  Vec<TriggerHolder>}`; `TriggerHolder { card: CardInstance, .. }`; `SettleOptions: Default`.
- `prompts::{open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>, close_prompt(&mut
  EngineSink) -> Option<PendingChoice>, resume_self(&EffectContext, &str, IndexMap<String, Value>) ->
  Resume, resume_at(<args deserialisable from { defId, step }>) -> Resume}`; `OpenPromptArgs { player,
  kind, aim, prompt, options, min, max, budget, owner, resume }` with TS's optional fields as `Option`.
- `modifiers::schedule_delayed(&mut EngineSink, PlayerId, DelayedAt, Resume, watch: Option<String>,
  not_before: Option<i32>) -> DelayedEffect`.
- `work::owed_work(&GameState, Option<&str>) -> Vec<WorkItem>` (owned: the tests round-trip and compare
  the item).
- `catalog::{registered_catalog() -> &CardDefs, def_of(Option<&GameState>, &str) -> &CardDef (or
  owned)}`; `scripts::{registered_scripts() (`.clone()`d into an `IndexMap<String, CardScripts>`),
  script_of(&GameState, &str) -> CardScripts}` (SURFACE §6.6); testkit `register_catalog(CardDefs)`,
  `register_scripts(IndexMap<String, CardScripts>)`.
- `brittle_count::active_brittle_count(&CardInstance) -> Option<i32>`; `params::step_param(&mut
  CardInstance, &str, i32)`; `subsystems::twice_forward::{twice_forward_plays(&CardInstance) -> i32,
  TWICE_FORWARD_PLAYS_KEY: &str}`.
- `effects::{damage, summon, draw}` taking one argument struct that deserialises from the TS literal
  (`json_as(json!(…))`).
- SURFACE §6.1's `reduce`, `begin_game`, `view_for`, `hash_state`.

## Decisions
- TS `sinkFor(state)` cannot return a sink that owns its rng and still lets the test read `state`, so
  each file has a private `SinkFor { events, rng }` (rng `Rng::new(&state.seed, state.rng_cursor)` at the
  moment TS made the sink) whose `on(&mut state)` builds an `EngineSink` per call. One `SinkFor` per TS
  sink: calls on the same TS sink share its event list and rng; the cursor is never written back (TS did
  not). TS's `sink.events` / `sink.state` reads become `bench.events` / `state` between calls.
- Instances: `put`, `in_hand` and `new_instance` hand back clones; every TS read or write through the
  live object goes through `find_instance`/`find_instance_mut` by id (`card`, `card_mut`, `trap_of`,
  `trap_of_mut` helpers). `place_on_field` gets `&mut card` (TS mutated the card it placed).
- Fixture defs are built with `json_as(json!(TS literal))`; TS's `nextIndex` module counters are
  written out as the index each def gets in TS's creation order. TS's `{ ...def, type }` spreads are
  struct-update syntax on `CardDef`.
- Fixture scripts are `Script { …, ..Script::default() }` literals with `hook`, `aura_hook`,
  `TriggerDef::new(…).with_when(…)`; `resume` maps as `[(step, hook)].into_iter().collect()`;
  `staticFlags` as `json_as(json!({ "castOnDraw": true }))`.
- Actions are TS `ActionInput` literals in `json!`, the nonce added by a local `act`/`act_result` and
  deserialised with `json_as::<Action>`. TS's module-level `let nonce` is a `static AtomicU32` per file
  (clippy.toml bans no atomic); hashes exclude `applied`, so the interleaving of parallel tests changes
  nothing a test reads.
- Events built by hand are `GameEvent` struct variants (frozen); event field reads match `GameEvent`
  variants; `toEqual` on event lists compares `serde_json::to_value` with the TS literal in `json!`.
- `toMatchObject` on a view asserts the fields it names; `toEqual` on a `TrapDispatch` asserts
  `fired` and `paused`; `toEqual([])` on a list without `Debug`/`PartialEq` (holders, matches) is
  `.is_empty()`.
- `toThrow(/did not settle in 100 passes/)` is a `catch_unwind` on the panic and a substring check on
  its message (SURFACE §4.4.9: a TS throw on an impossible state is a panic with the same message).
- `toMatch(/a prompt is open/)` is a substring check on the `error` text.
- `JSON.parse(JSON.stringify(x))` is a serde round trip through `serde_json::Value`.
- `indexOf` keeps TS's -1 for a missing event; `Math.max`/`Math.min` of an empty list keep ∓∞
  (`i64::MIN`/`i64::MAX`); `Number(id.slice(1))` keeps NaN for an id that is not a number.
- `scriptOf(card).cry` (an unradiant card) is `script_of(&state, &card.def_id).base.cry`, by SURFACE
  §6.6's `script_of(state, def_id) -> CardScripts`.
- Test names by SURFACE §7.3: every `R<n>` in an `it` title is a leading `r<n>_` token, in title order
  (`"R212 … (R171)"` → `r212_r171_…`); `§x.y` becomes `sx_y`; `#74` becomes `c74`, `C+` `c_plus`. `mod`
  names keep the `describe` title's words in order (their `r<n>` tokens included).
- `assert!(CRY_ON_PLAY_ONLY)` carries `#[allow(clippy::assertions_on_constants)]` on its test: the TS
  test asserts the constant on purpose.
