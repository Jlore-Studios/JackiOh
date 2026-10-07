# Slice: part 25 (engine tests 2), chunk 4 of 7
BUILDS-RUN: 0

## FILES
- `crates/engine/tests/rules/prompt_kinds.rs` ← `packages/engine/test/prompt-kinds.test.ts` (27 tests, 8 mods)
- `crates/engine/tests/rules/prompts.rs` ← `prompts.test.ts` (18 tests, 1 mod; its fixture defs and scripts stay in the file, as in TS)
- `crates/engine/tests/rules/quests.rs` ← `quests.test.ts` (27 tests, 7 mods)
- `.fullsend/notes/spec-gaps-part-25-4.md` (assertions written in a Rust form; none dropped)

Every TS `it` is one `#[test]`, every `describe` one `mod`, in TS order, with the header comment and
every rule-stating comment kept. Ruling ids in a title lead the Rust name, in the order they appear
(`"E18 R79 …"` → `r79_e18_…`, `"… (R81) …"` → `r81_…`; SURFACE §7.3); `§10.6` → `s10_6`, `M3-T3` →
`m3_t3`, `Classic+ #42` → `classic_plus_42`.

## GAPS
No test could not be ported. Names these files call that other parts provide (TS name snake_cased at
its TS module's Rust path, SURFACE §4), with the shapes the calls assume:

Fixtures (part 24). Their files were on `staging` before this chunk pushed (test files, not
`crates/*/src/`), so the calls match them as written there:
- `prompt_harness::{board, cast_now, answer_keys (-> AnswerResult { events, rng, error }), open_as (owned
  PendingChoice), round_trip, act(&GameState, impl Serialize, Option<&mut Vec<Action>>) -> GameState,
  replayable(&str, &[&str], &[&str]) -> Replayable { state, log, decks }, expect_replays, hand_card}`.
- `harness::{new_game(&str, Option<decks>), setup_catalog, put(.., Value), slot, in_hand(.., i32),
  set_library(.., &[impl AsRef<str>]), events_of_type(&[GameEvent], impl ToString) -> Vec<GameEvent>}`;
  `catalog::vanilla_deck(i32, i32)`; `combat::plain` (a `LazyLock<CardDef>` static: `plain.id`);
  `scripts::{cn_virus, going_long, x_bolt}()`.
- `prompts::{prize, pickle, glitch, numberer, quiz, papaya, quest, acquire, back_from_gy, mind_melt,
  cross_pick, mill, grunt}()`, `quickdraw_of(&CardDef)`, `PICKLE_OPTIONS`, `GLITCH_OPTIONS`, `QUIZ { statement,
  options, correct: i32 }`, `QUEST_REWARDS [{ id, label }]`, `BACK_BUDGET: i32`.
- `quests::{tree, draw_one, draw_two, draw_then_recruit, slay, banish, banish_any, bolt, mill, limiter, blank,
  recall, bless}()`, `register_quest_fixtures()`, `GoalKind = &'static str`, `GOALS` and `GOAL_CARDS` (indexed
  by the TS key, `GOAL_CARDS["draws"]`), `TREE: LazyLock<QuestBook>`, `ONLY_QUEST`, `TREE_HEAL`, `TREE_PING`,
  `TREE_BUFF`, `BOLT`, `RECALL: i32`.

Engine (parts 2–8), by module:
- `prompts`: `run_hook_resumable(&mut EngineSink, &CardInstance, &str, <options: Default>) -> bool`,
  `open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>` with `OpenPromptArgs { player, kind,
  aim, prompt, options, min, max, budget, owner, resume }`, `answer_prompt(&mut EngineSink, &AnswerInput) ->
  Result<_, EngineError>`, `why_answer_refused(&PendingChoice, &AnswerInput) -> Result<(), EngineError>`
  (both TS refusal strings, SURFACE §4.4.9; read as `.err().map(|e| e.to_string())`), `AnswerInput {
  player_id, choice_id, selection }`, `prompt_answers(&PendingChoice) -> Vec<ActionBody>`,
  `hero_option_label(PlayerId, PlayerId)` and `cell_option_label(PlayerId, Row, i32, PlayerId)` (`String`),
  `answer_key_of(&IndexMap<String, Value>) -> Option<_: ToString>`, `ANSWER_KEY: &str`, `PROMPT_KINDS:
  &[PromptKind]`.
- `effects`: `ANSWER_OPTION_IDS: &[&str]`; `choose_mode`, `choose_target`, `choose_from_hand`,
  `discover_from_catalog`, `discover_from_graveyard`, `remember`, `add_to_hand`, `damage`, each taking its
  argument struct from the TS literal (`json_as(json!(…))`); `chosen_options(&EffectContext) -> Vec<String>`.
- `resolve`: `make_context(EngineSink, Option<CardInstance>, HookOptions) -> EffectContext`, `HookOptions:
  Default { controller: Option<PlayerId>, .. }`.
- `reduce::{reduce, begin_game, legal_actions, ReduceResult}`; `replay::{hash_state, fold, FoldArgs: Default {
  seed, decks, log, .. }}`; `view_for::view_for`; `setup::mulligan_prompt_for(&GameState, PlayerId) ->
  Option<PendingChoice>` (or `Option<&PendingChoice>`: both compile); `work::can_resume(&Resume) -> bool`;
  `triggers::settle(&mut EngineSink, <options: Default>)`; `draw::draw(&mut EngineSink, PlayerId, i32)`
  (called by its full path); `layers::unit_view(&GameState, &CardInstance) -> UnitView { keywords, .. }`.
- `subsystems::quests`: `quest_memory_of(&CardInstance) -> Option<QuestMemory>` (`QuestMemory: Serialize`,
  camelCase: the tests read it as JSON), and `quest_book_of(&GameState, &CardInstance) -> Option<QuestBook>`.
  TS's `questBookOf(card)` takes the card alone; Rust needs the state, since a fused card's script is
  composed from `state.transient_defs` on lookup (SURFACE §6.6) and R102's tests ask a fusion's book.
- `subsystems::fuse`: `fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>`, `FuseArgs: Default {
  ingredients: Vec<CardInstance>, target: Option<CardInstance>, .. }`.
- testkit: `register_catalog(CardDefs)`, `register_scripts(IndexMap<String, CardScripts>)`,
  `registered_catalog()`, `registered_scripts()` (both cloned before extending).

## Decisions
- A TS `sinkFor(state)` is a sink built in the test over local events and rng (`EngineSink::new`;
  `prompts.rs`'s `with_sink`, which runs the test's sink steps in a closure and reads the state through
  `sink.state`). `harness::sink_for` leaks its rng and events; it is not needed. Where TS never wrote the
  sink's cursor back to `state.rngCursor`, the port does not either.
- TS's module counters: `prompts.test.ts`'s `nextIndex` (from 1400) is written out per card
  (1401…1408, TS order); the nonce counters (`pr<n>`, `qt<n>`) are a file-level `AtomicU32`.
- `toBe` object identity on a prompt or a refused result's state is `==` (Rust holds copies); `expect(
  sink.state).toBe(state)` is `std::ptr::eq` on the sink's state and the board it was built over.
- `deepValues(…)` "no value is a function" is ported as "the value is plain data": it serialises and
  reads back equal (`expect_plain_data`), since a closure cannot be a field of a serde type.
- `const { sink } = answerKeys(…)` reads `AnswerResult.events` (the fixture hands back what the sink held).
- `expect(() => act(…)).toThrow()` is `reduce(…).error.is_some()`: the harness's `act` throws exactly when
  `reduce` refuses.
- Writes TS made through a held card object (`card.memory.quest = …`, `stolen.owner = …`, `wall.damage = 0`,
  `three.costOverride = 3`, `hand[2].radiant = true`) go through `find_instance_mut` on the state.
- R98 "no self": TS checked that the dropped card object's memory stayed empty. The port checks the
  dropped clone, that the card is in no zone, and that nothing in the serialised state carries `sawSelf`.
- Local helpers keep TS's names (`act`, `play`, `hand`, `end_turn`, `board`); a local item shadows the
  testkit's glob, which is legal Rust.
- `quests.test.ts`'s `GOALS.draws.count` and friends are read off the goal's JSON (`goal_number`).
