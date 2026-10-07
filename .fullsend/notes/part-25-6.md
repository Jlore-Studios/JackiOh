# Slice: part 25 (engine tests 2), chunk 6 of 7 (#413)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
- `crates/engine/tests/rules/rulings_c.rs` ← `packages/engine/test/rulings-c.test.ts` (55 tests, 15 mods)
- `crates/engine/tests/rules/scorer.rs` ← `packages/engine/test/scorer.test.ts` (9 tests, 1 mod)
- `crates/engine/tests/rules/start_of_opponent_turn.rs` ← `packages/engine/test/start-of-opponent-turn.test.ts`
  (1 test, 1 mod)
- `.fullsend/notes/spec-gaps-part-25-6.md` (partial assertions, none dropped)

All three were empty placeholders on `staging`. Every TS `it` is one `#[test]`, every `describe` one `mod`,
in TS order, with the header comment and every rule- or ruling-citing comment kept (the R-ids and §-ids of
each Rust file equal its TS file's, checked by script; R134, R135 and R763 appear only in TS titles and so
only as `r<n>_` tokens). Ruling ids in a title lead the Rust name (SURFACE §7.3); a `describe` title is its
`mod` name snake-cased (`SPEC §11 R91–R96: …` → `spec_11_r91_r96_…`), `§8 #97 …` → `spec_8_97_…`.

## GAPS
Tests not ported: none (see `spec-gaps-part-25-6.md` for the assertions written against the nearest Rust
observable).

Names these files call that other parts provide (TS name snake_cased at its TS module's Rust path, SURFACE
§4). Where the owning part's notes give a signature I followed it; the rest are guesses.

Fixtures (part 24 chunk 6, `crate::rules::fixtures::…`, matched to its notes):
- `harness::{new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState, put(&mut GameState, &str,
  ZoneSlot, Value) -> CardInstance (options json!({}) / json!({ "radiant": true })), slot(PlayerId, Row, i32)
  -> ZoneSlot, in_hand(&mut GameState, &str, PlayerId, i32) -> Vec<CardInstance>}`; `combat::stacker`
  (a `LazyLock<CardDef>` static, read as `stacker.id`). `sink_for` and `events_of_type` are not called (see
  Decisions).

Engine (parts 2–8):
- `combat::{switch_position(&mut EngineSink, &CardInstance, SwitchPositionOptions { to, spend_exertion }:
  Default) -> Result<_, EngineError>, declare_attack(&mut EngineSink, &CardInstance, &AttackTarget) ->
  Result<_, EngineError>, force_attacks_on(&mut EngineSink, &[CardInstance], &AttackTarget, Option<u32>),
  has_exertion(&GameState, &CardInstance, ExertionKind::Switch) -> bool, AttackTarget::{Unit { instance },
  Hero { player }}}` (part 3.1).
- `damage::{deal_damage(&mut EngineSink, DamageArgs { source, target, amount, flags }) -> i32,
  DamageTarget::{Unit { instance }, Hero { player }}, hero_damage_cap(&GameState, PlayerId) -> Option<i32>,
  lose_health(&mut EngineSink, PlayerId, i32) -> i32}`.
- `draw::{draw_one(&mut EngineSink, PlayerId, None) -> DrawOutcome (::Fatigue), add_to_hand(&mut
  EngineSink, &mut CardInstance) -> AddToHandOutcome (::Hand)}` (part 4.2).
- `layers::{unit_view, stats_with_buffs (.attack, .max_health), unit_has(.., KeywordKind)}`;
  `mana::printed_cost(&GameState, &CardInstance) -> i32`; `catalog::{def_of(Option<&GameState>, &str) ->
  &CardDef, def_by_index(SetName, &str) -> Option<_>, registered_catalog() -> &CardDefs}`;
  `scripts::{scripts_for(&GameState, &str) -> CardScripts (or &), registered_scripts() ->
  IndexMap<String, CardScripts>}` (part 2).
- `zones::{active_units_of, card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>, first_free_zone(&GameState,
  PlayerId, Row) -> Option<ZoneSlot>, place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot,
  PlaceOnFieldOptions { stack }), remove_from_any_zone(&mut GameState, &mut CardInstance),
  move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone::Library, MoveToZoneOptions::default())}`.
- `play_choices::{SHEEP_TOKEN_INDEX, PlayAction (Deserialize from `{ "type": "play", … }`, Serialize to the
  same JSON: its `tributes`/`targets` are read as JSON), why_choices_refused(&GameState, PlayerId,
  &CardInstance, &PlayAction) -> Result<(), EngineError>, tribute_cost_of(&GameState, &CardInstance),
  may_tribute_enemy_units(&GameState, &CardInstance), declared_targets(&GameState, &CardInstance) ->
  Vec<TargetDecl>, tribute_value_of, legal_tribute_units -> Vec<CardInstance or &>, legal_tribute_sets ->
  Vec<Vec<String>>, legal_zones_for(.., &[String]) -> Vec<ZoneChoice>, play_actions_for -> Vec<PlayAction>}`
  (part 4.2's shapes: the state argument on the card readers is theirs).
- `play_steps::PLAY_WORK_KIND`; `traps::{TRAP_FIRING_WORK, traps_watching(&GameState, &GameEvent) ->
  Vec<TrapMatch { trap, .. }>, fire_traps_for / run_trap_window(&mut EngineSink, &GameEvent) ->
  TrapDispatch { fired: Vec<String>, paused: bool }, is_trap_window_event(&GameEvent) -> bool}`.
- `prompts::{RESUME_HOOK, open_prompt(&mut EngineSink, OpenPromptArgs { player, kind, aim, prompt, options,
  min, max, budget, owner, resume }), resume_self(&EffectContext, &str, IndexMap<String, Value>) -> Resume,
  answer_prompt(&mut EngineSink, &AnswerInput { player_id, choice_id, selection }) -> Result<(), EngineError>,
  run_resume(&mut EngineSink, &Resume, ResumeOptions { controller, .. }: Default) -> bool}`.
- `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext,
  HookOptions { controller, targets, .. }: Default, apply_effects(&[Effect], &mut EffectContext),
  cast_card(&mut EngineSink, &mut CardInstance, CastOptions::default()),
  flag_return_to_hand_at_end_of_turn(&mut GameState, &str), HookName::{StartOfTurn, EndOfTurn, OnPlayHook}}`.
- `modifiers::schedule_delayed(&mut EngineSink, PlayerId, DelayedAt, Resume, Option<String>, Option<i32>)`;
  `turn::{start_turn(&mut EngineSink, PlayerId), end_turn(&mut EngineSink)}`; `triggers::{settle(&mut
  EngineSink, SettleOptions::default()), trigger_holders_with_hook(&GameState, HookName, Option<PlayerId>)
  -> Vec<TriggerHolder { card, .. }>}`; `work::{begin_work_cascade, push_work(&mut EngineSink, Resume,
  Option<PlayerId>) -> WorkItem, take_work(&mut GameState) -> Option<WorkItem>, run_work_item(&mut
  EngineSink, &WorkItem)}` (panics with an R113 message naming the hook); `query::played_ids_this_turn`;
  `view_for::{view_for, HIDDEN_ID}`; `reduce::{reduce, begin_game}`; `effects::delay::DELAYED_HOOK`.
- `subsystems::activate::{ACTIVATIONS_MEMORY_KEY, ActivateAction (Deserialize from `{ "type": "activate",
  "instanceId" }`), activate_ability(&mut EngineSink, PlayerId, &ActivateAction) -> Result<(), EngineError>,
  why_cannot_activate_ability(&GameState, PlayerId, &str, Option<&str>) -> Result<(), EngineError>}`;
  `subsystems::combo_index::{grade_of(&CardInstance) -> i32, grade_rises(&GameState, &CardInstance) -> bool,
  played_cards_this_turn(&GameState, PlayerId)}`; `subsystems::fuse::{FuseArgs (Deserialize from `{
  ingredients, target }`), fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>}`;
  `subsystems::hero_power::{HERO_POWERS: &[HeroPower { name, x, targets: Option<fn() -> Vec<TargetDecl>> }],
  HERO_POWER_NAMES (Serialize), POWER_KEY, POWER_RESUME, RUSH_TOKEN_INDEX, STEADY_SHOT_PARAM, hero_power (a
  hook fn), power_abilities(bool), power_ability_of(&GameState, &CardInstance) -> Option<ActivationDecl>,
  power_of(&CardInstance) -> Option<HeroPower>, roll_power() -> Effect, used_this_turn(&GameState,
  &CardInstance) -> bool}` (part 8.2/8.3's shapes); `subsystems::scorer::{ScorePriority::{Lethal, Clear, Heal,
  Value}, Scored { def, score: f64, priority, parts { kills: f64, clear: f64, .. } }: Clone, ScorerOptions {
  radiant }, ZEPHYRS_INDEX, candidate_defs(), compare_scored(&Scored, &Scored) -> Ordering,
  projected_board_damage(&GameState, PlayerId) -> i32, rank / top_three(&GameState, PlayerId, &ScorerOptions)
  -> Vec<Scored>}`.
- Effect verbs, each built with `json_as(json!(<TS literal>))` (so each argument type must derive
  `Deserialize`): `damage, damage_all, discard_random, discard_hand, exile, exile_hand, flip_coins,
  forced_attacks, steal`; `exile_matching` and `draw_from_library` only named.

## Decisions
- TS's live objects: `put`, `in_hand` and `new_instance` hand back copies, so a read after the engine ran
  looks the card up by id (`live`) and a write goes to the card in the state (`live_mut`); a copy of a
  card the test took off the board (`removeFromAnyZone` then `x.zone = …`) keeps TS's local zone write.
- TS's `sinkFor(state)` aliased the state. `SinkFor` (local) keeps a sink's events and rng (`Rng::new(&seed,
  rng_cursor)`, as `sinkFor` did) and lends them to `EngineSink::new` for each call, so one sink's rng
  cursor and event list carry across calls as TS's object did while the test reads the state between
  calls. The harness's `sink_for` (a fresh rng per call) is not used for that reason.
- `eventsOfType` narrowing: local `filter_map`s over the `GameEvent` variants (`hits`, `trap_fired_ids`,
  `attackers_declared`, `count_of` via `event_type()`).
- `whyX` string refusals are read as `refusal(Result) -> Option<String>`; `toMatch(/literal/)` is
  `contains`; `toThrow(/x/)` is `panic_text(catch_unwind)` + `contains`.
- Fixture defs are TS's object literals as `json!` with the `{ ...defaults, ...extra }` spread written out
  (`spread`), each def's index the one TS's `nextIndex` counter gave it (1901–1948 in declaration order);
  the catalog is extended in TS's `DEFS` order. Scorer and start-of-opponent-turn defs likewise.
- Names that are enums or strings depending on the port (`HeroPower.name`, `HERO_POWER_NAMES`, a filter, a
  `PlayAction`) are compared as JSON (`as_json`), so either shape passes.
- TS's module `let nonce` is a `thread_local!` `Cell<u32>` (each `#[test]` its own thread).
- R102's second fusion is handed the consumed ingredient's copy with its zone set to `gone`, which is what
  TS's shared object held after the first fusion (R86).
- I read part 24.6's notes for the fixture signatures and grepped the four `pub fn`/`pub static` signature
  lines of `fixtures/harness.rs` and `fixtures/combat.rs` to confirm them (test fixtures, not engine
  source); no engine source under `crates/*/src/` was read beyond part 1's frozen files.
