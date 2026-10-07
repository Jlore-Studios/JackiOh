# Slice: part 24 (engine tests 1: effect verbs and fixtures), chunk 7 of 7 (#412)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

TESTS-IN: 0. The twelve TS files are fixtures: none has a `describe` or an `it`, so there is no test to
port and nothing for a spec-gaps file. `promptHarness.ts`'s two `expect` calls (`openAs`,
`expectReplays`) are `assert_eq!`/`assert!` in the Rust helpers.

## FILES
All twelve were empty placeholders on `staging`; each is the whole TS file (every export and local
helper, the header and every comment that states a rule or cites a ruling). No `todo!`,
`unimplemented!`, `#[ignore]` or `// TODO`.
- `crates/engine/tests/rules/fixtures/ky_test.rs` ← `kyTest.ts`
- `crates/engine/tests/rules/fixtures/last_boards.rs` ← `lastBoards.ts`
- `crates/engine/tests/rules/fixtures/papaya.rs` ← `papaya.ts`
- `crates/engine/tests/rules/fixtures/play_pipeline_a.rs` ← `playPipelineA.ts`
- `crates/engine/tests/rules/fixtures/play_pipeline_b.rs` ← `playPipelineB.ts`
- `crates/engine/tests/rules/fixtures/prompt_harness.rs` ← `promptHarness.ts`
- `crates/engine/tests/rules/fixtures/prompts.rs` ← `prompts.ts`
- `crates/engine/tests/rules/fixtures/quests.rs` ← `quests.ts`
- `crates/engine/tests/rules/fixtures/rng_child.rs` ← `rng-child.ts`
- `crates/engine/tests/rules/fixtures/scripts.rs` ← `scripts.ts`
- `crates/engine/tests/rules/fixtures/turn.rs` ← `turn.ts`
- `crates/engine/tests/rules/fixtures/twice_forward.rs` ← `twiceForward.ts`

## SURFACE (what these files export; the other test porters call these)
- Every file with definitions or scripts has the brief's `pub fn catalog() -> CardDefs` (this file's
  definitions by id) and `pub fn scripts() -> IndexMap<String, CardScripts>` (this file's scripts by id).
  `prompt_harness` and `rng_child` hold neither.
- A TS lowercase value export is a zero-argument function of its snake_cased name returning a fresh
  value: every `export const x: CardDef` is `pub fn x() -> CardDef` (`ky_test()`, `coin()`, `prize()`,
  `bolt()`, `forward()`, `log_card()`, `second_wind()` …), `acquireScripts`/`mindMeltScripts`/
  `fluffyGripScripts`/`rewindScripts` are `acquire_scripts()` … `-> CardScripts`.
- A TS UPPER_CASE export keeps its name: string/number constants are `pub const` (`PORTAL`, `TRAP`,
  `FIELD_TRAP`, `PORTAL_RADIANT_CARDS: i32`, `TURNER_DAMAGE`, `CRIER_DAMAGE`, `LOG_LANE: usize`,
  `QUEST_REWARD_KEY`, `RANDOM_POOL`/`DISCOVER_POOL`/`PICKLE_OPTIONS: &[&str]`, `PAPAYA_CELLS: usize`,
  `BACK_BUDGET`, `ONLY_QUEST`, `TREE_HEAL`, `TREE_PING`, `TREE_BUFF`, `BOLT`, `RECALL: i32`), computed ones
  `pub static X: LazyLock<…>` (`FIXTURE_BANK: Vec<KyTestProblem>`, `LB_DECKS: (Vec<String>,
  Vec<String>)`, `PA: Pa`, `PA_DEFS`, `PB_DEFS`, `PROMPT_DEFS`, `QUEST_DEFS`, `TURN_DEFS`, `FIXTURE_DEFS`,
  `PAPAYA_DEFS: Vec<CardDef>`, `PB_SCRIPTS`, `PROMPT_SCRIPTS`, `TURN_SCRIPTS`, `FIXTURE_SCRIPTS`,
  `TWICE_FORWARD_SCRIPTS: IndexMap<String, CardScripts>`, `GLITCH_OPTIONS: Vec<String>`, `TREE:
  QuestBook`, `GOALS: IndexMap<GoalKind, QuestGoal>`, `GOAL_CARDS: IndexMap<GoalKind, CardDef>` with
  `GoalKind = &'static str`). Each `*_SCRIPTS`/`*_DEFS` static also has a call alias for callers who
  wrote the constant as a function (`fixture_scripts()`, `fixture_defs()`, `turn_scripts()`,
  `turn_defs()`, `prompt_scripts()`, `prompt_defs()`, `pb_scripts()`, `pb_defs()`,
  `twice_forward_scripts()`, `papaya_scripts()`, `quest_scripts()`), as part 24.3's notes call
  `fixture_scripts()`.
- `PA` is a `pub struct Pa` with one `pub` `CardDef` field per TS key, snake_cased (`PA.bolt.id`,
  `PA.counter_trap2`, `PA.q_bolt`, `PA.ai_card`). `QUIZ` is `pub const QUIZ: Quiz { statement, options:
  &[&str], correct: i32 }`; `QUEST_REWARDS: &[RewardOption { id, label }]`. `HERO_POWERS` is `pub use
  subsystems::hero_power::HERO_POWER_NAMES as HERO_POWERS` (TS `export const HERO_POWERS =
  HERO_POWER_NAMES`).
- Functions, with TS's default arguments written out (Rust has none): `register_ky_test_fixtures()`,
  `register_last_boards()`, `register_papaya_fixtures()`, `register_play_a()`, `with_play_a(GameState)
  -> GameState`, `register_pipeline_b()`, `register_prompt_fixtures()`, `register_quest_fixtures()`;
  `fixture_catalog(CardDefs)`, `turn_catalog(CardDefs)`, `twice_forward_catalog(CardDefs) -> CardDefs`;
  `turn::{notes(&GameState) -> Vec<String>, write(&mut GameState, &str), note(impl Into<String>) ->
  Effect, ask_controller(&str) -> Effect}`; `quickdraw_of(&CardDef) -> CardDef`;
  `last_boards::{act(&GameState, &mut Vec<Action>, Value) -> GameState, portal_game(&str,
  Option<LastBoardInput>) -> PortalGame { seed, state, log }}`; `play_pipeline_b::{pb_reduce(&GameState,
  impl Serialize) -> ReduceResult, pb_act(&GameState, impl Serialize) -> GameState, pb_playing(&str) ->
  GameState, in_graveyard(&mut GameState, &str, PlayerId) -> CardInstance, only<T: Clone>(&[T]) -> T,
  plays_of(&GameState, &str, PlayerId) -> Vec<ActionBody>, round_trip(&GameState) -> GameState}`;
  `prompt_harness::{board(&str) -> GameState, resolving_card(&mut GameState, &str, PlayerId, bool) ->
  CardInstance, cast_now(&mut GameState, &str, PlayerId, bool) -> SinkResult { events, rng } (derefs to
  its `Vec<GameEvent>`), must<T>(Option<T>, &str) -> T, answer_keys(&mut GameState, &[&str]) ->
  AnswerResult { events, rng, error: Option<String> }, open_as(&GameState, PromptKind, PlayerId) ->
  PendingChoice, round_trip, event_types(&[GameEvent]) -> Vec<String>, act(&GameState, impl Serialize,
  Option<&mut Vec<Action>>) -> GameState, replayable(&str, &[String], &[String]) -> Replayable { state,
  log, decks }, expect_replays(&str, &(Vec<String>, Vec<String>), &[Action], &GameState),
  hand_card(&GameState, PlayerId, &str) -> CardInstance}`; `rng_child::rng_child(&[&str]) -> String`.
  An action body is `impl Serialize` where TS took `ActionInput`, so an `ActionInput` and its `json!`
  literal both work; `last_boards::act` takes the `Value` TS typed as a record.

## GAPS
No test was left out. Names called that another part provides (TS name snake_cased at its TS
module's path; the shape assumed):

Fixtures (part 24 chunk 6):
- `fixtures::catalog::{spell_def(index, overrides: Value) -> CardDef, unit_def(index, overrides: Value)
  -> CardDef, vanilla_deck(size: i32, from: i32) -> Vec<String>}` (index an integer literal; size as
  `DECK_SIZE - n`, an `i32`).
- `fixtures::harness::{setup_catalog(), new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState}`
  (the shape parts 24.1–24.4 call). Its `setupCatalog` needs `fixtures::scripts::{fixture_catalog,
  FIXTURE_SCRIPTS (or fixture_scripts()/scripts())}` from here.

Engine:
- `catalog::registered_catalog()` (`&'static CardDefs`) and `scripts::registered_scripts()` (owned or a
  reference: both are `.clone()`d) must answer the testkit's override when one is set: every
  `register…` here merges onto them, as TS spread `registeredCatalog()`/`registeredScripts()`.
  `catalog::def_of(Option<&GameState>, &str) -> &CardDef` (part 2.2's shape; `.type_` read).
- `testkit::{register_catalog(CardDefs), register_scripts(IndexMap<String, CardScripts>)}` (part 5).
- `prompts::{open_prompt(&mut EngineSink, OpenPromptArgs { player, kind: PromptKind, aim: None,
  prompt: String, options: Vec<PromptOption>, min: None, max: None, budget: None, owner: None,
  resume: Resume }), resume_self(&EffectContext, &str, IndexMap<String, Value>) -> Resume,
  run_hook_resumable(&mut EngineSink, &CardInstance, &str, HookResumableOptions { controller:
  Option<PlayerId>, .. }: Default), answer_prompt(&mut EngineSink, &AnswerInput { player_id, choice_id,
  selection }) -> Result<_, EngineError>}` (part 3.2's names).
- `triggers::{settle(&mut EngineSink, SettleOptions), SettleOptions: Default}` (part 3.3's shape).
- `damage::{deal_damage(&mut EngineSink, DamageArgs { source: Option<CardInstance>, target, amount: i32,
  flags: None }), DamageTarget::{Unit { instance }, Hero { player }}}`.
- `zones::{active_units_of(&GameState, PlayerId) (each element cloned with `CardInstance::clone`, so owned
  or borrowed both work), card_at(&GameState, &ZoneRef) -> Option<&CardInstance>, move_to_zone(&mut
  GameState, &mut CardInstance, OffFieldZone::Graveyard, Default::default())}` (part 2.1's shapes).
- `query::last_spell_played(&GameState) -> Option<PlayRecord>`; `replay::fold(&FoldArgs)` with `FoldArgs:
  Deserialize` (built with `json_as` from `{ seed, decks, log }`) and a result with `state` and `errors`
  (`.is_empty()`); `reduce`, `begin_game`, `create_game(&CreateGameOptions)`, `legal_actions`,
  `hash_state` as SURFACE §6.1.
- Subsystems: `ky_test::{ky_test_script(&[KyTestProblem]) -> KyTestScript { cry: Hook, resume:
  IndexMap<&'static str, Hook> }, KyTestProblem: Deserialize}` (part 8.2's notes);
  `papaya::{PAPAYA_STEP: &'static str, papaya_begin() -> Vec<Effect>, papaya_answered(ctx) -> Vec<Effect>}`;
  `twice_forward::twice_forward_trigger(args) -> TriggerDef` with `args` deserialisable from TS's
  `{ radiantCopy }` (built with `json_as`); `hero_power::{HERO_POWER_NAMES (Serialize, TS's name strings;
  a const slice or a static: read as `&*HERO_POWERS`), POWER_RESUME: &'static str, STEADY_SHOT_PARAM,
  hero_power(ctx) -> Vec<Effect>, power_abilities(bool) -> Vec<ActivationDecl>}`; `quests::{held_quest_auras(
  &CardInstance) -> Vec<String>, hold_quest_aura(&str) -> Effect, open_quest(&str) -> Effect,
  quest_def_of(&QuestBook, &str), quest_reward_of(&QuestBook, &str)}` (an `Option` of a reference or of
  an owned value: both read).
- Effects, every argument built with `json_as(json!(TS literal))` so the argument structs' Rust names do
  not matter: `damage`, `heal`, `draw`, `exile`, `destroy`, `discard`, `buff`, `summon`, `add_to_hand`,
  `plague`, `sacrifice`, `discard_hand`, `next_turn_mana`, `gain_mana`, `shuffle_copies_of_self`,
  `remember_random`, `end_turn_after_actions`, `delay`, `destroy_at_next_turn_start`,
  `discard_hand_at_turn_end`, `for_rest_of_game`, `add_cost_rule`, `enchant_next_spell`,
  `add_player_modifier`, `discover_from_catalog`, `discover_from_last_board`, `add_from_last_board`,
  `add_random_from_last_board`, `choose_mode`, `choose_target`, `choose_reward`, `choose_from_hand`,
  `choose_cost_in_hand`, `choose_number`, `choose_answer`, `choose_cell`, `choose_pick`, `exile_matching`,
  `give_from_hand`, `take_from_library`, `trigger_cry`, `exile_bottom_of_library`,
  `return_random_from_graveyard`, `buff_random_unit`. TS calls with no argument to a `(args = {})`
  verb pass `Default::default()`: `add_random_from_graveyard`, `end_turn`, `counter_play`, `recruit`,
  `draw_from_opponent`. `summon_this()` takes none (TS took none). Readers: `chosen_options(&EffectContext)
  -> Vec<String>`, `chosen_number -> Option<i32>`, `chosen_cells -> Vec<ZoneRef>`, `answered_correctly ->
  bool`.
- The three cast verbs hold functions, so they are built as struct literals (part 6.2's names):
  `CastNewArgs { def: CastNewDef (From<String>), radiant: Option<bool>, how: CastHow: Default }`,
  `CastRandomArgs { query: CastRandomQuery::Fixed(CatalogQueryArgs), count:
  Option<CastRandomCount::Fixed(i32)>, radiant: Option<bool>, how: <Deserialize> }`, `CastEachArgs {
  cards: CastEachCards = Arc<dyn Fn(&EffectContext) -> Vec<String> + Send + Sync> (a fn item is
  passed), how: <Deserialize> }`. Each is behind one private helper per file, so a different layout is
  one edit.

## Decisions
- Definitions are the TS object literals through `json_as::<CardDef>`, with a private `spread` helper
  for TS's `{ ...base, ...extra }`. TS numbered them from a module counter mutated as the module loaded;
  each index is written out in the order TS made them (kyTest 3301–3308, quests 3301–3322 with the nine
  goal cards first, prompts 3101–3132, turn 4301–4325, pipeline B 4521–4557, pipeline A 5001–5036).
- TS's module `let nonce` counters are one `thread_local!` `Cell<u32>` per file (a test runs on its own
  thread); the nonce text is TS's (`lb<n>`, `pm<n>`, `pb<n>`).
- TS's sink-returning helpers (`castNow`, `answerKeys`) build their own `EngineSink` over locals (an
  `EngineSink` borrows the state, so it cannot be returned) and answer its events and rng, writing the
  rng cursor back to the state as TS did. `SinkResult` derefs to its events (part 24.3 reads them as a
  `Vec<GameEvent>`; TS callers read `sink.events`).
- Where TS cast `ctx.event as Extract<…>` and read a field, the match keeps what a different event
  would do in TS (an absent field): `recurring` returns nothing, `quest-done` goes on to the reward,
  `snare`'s `when` holds, income tax's does not. A Field Spell's trigger keeps its condition in `run`.
- `String(ctx.data.x)` is a private `js_string` (`"undefined"` when absent). `typeof n === "number"`
  reads `Value::Number`.
- `rng-child.ts` was a child process reading `process.argv`; it is `rng_child(args) -> String`, the
  child's stdout text with TS's key order (`{"draws":…,"cursor":n}`), since the pure crates read no
  environment (SURFACE §3). Whoever ports `rng.test.ts` calls it in-process.
- `turn::LOG_LANE` and `prompts::PAPAYA_CELLS` are `usize` (an index and a length bound, SURFACE §4.3).
- The engine's `effects` module is named by path (`effects::damage(…)`), never globbed: its verbs
  collide with engine functions (part 1's testkit note).
