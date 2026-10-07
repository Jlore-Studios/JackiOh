# Slice: part 25 (engine tests 2), chunk 3 of 7 (#413, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
- `crates/engine/tests/rules/hero_power.rs` ← `packages/engine/test/heroPower.test.ts` (17 tests, 3 mods)
- `crates/engine/tests/rules/ky_test.rs` ← `kyTest.test.ts` (21, 3)
- `crates/engine/tests/rules/last_boards.rs` ← `lastBoards.test.ts` (15, 3)
- `crates/engine/tests/rules/modifiers.rs` ← `modifiers.test.ts` (13, 2)
- `crates/engine/tests/rules/papaya.rs` ← `papaya.test.ts` (24, 5)
- `crates/engine/tests/rules/pauses.rs` ← `pauses.test.ts` (11 of 13, 4 mods; 2 in spec gaps)
- `crates/engine/tests/rules/perfect_hand.rs` ← `perfectHand.test.ts` (14, 2)
- `.fullsend/notes/spec-gaps-part-25-3.md`

Every TS `it` is one `#[test]` (but pauses' two, below), every `describe` one `mod`, in TS order, with the
header comment and every rule-stating or ruling-citing comment kept. Ruling ids in a title lead the Rust
name (SURFACE §7.3); a title that starts with `§x.y` gets `sx_y_`, one that starts with `#n` gets `nn_`.

## GAPS

### Tests not ported (also in spec-gaps-part-25-3.md)
- `pauses.test.ts:354` "§9.3 resumes a three-step sequence at the step after the one that asked" and
  `:374` "§9.3 finishes a pause nested inside an owed sequence before the sequence's own tail": both drain
  work owed under a fixture hook that only `registerWorkHandler` made runnable (SURFACE §6.6 drops it).

### Names called that other parts provide (TS name snake_cased at its TS module's Rust path)
Fixtures (part 24, `crate::rules::fixtures::<file>`):
- `harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`, `setup_catalog()`,
  `sink_for(&mut GameState) -> EngineSink<'_>` (it owns or leaks its events and its rng, which starts at the
  state's cursor; the tests read `sink.state`, `sink.events`, `sink.rng` and pass `&mut sink` wherever
  `&mut EngineSink` is taken), `put(&mut GameState, &str, ZoneSlot, PutOptions) -> CardInstance` with
  `PutOptions { radiant: Option<bool> }: Default`, `slot(PlayerId, Row, i32) -> ZoneSlot`,
  `in_hand(&mut GameState, &str, PlayerId, usize) -> Vec<CardInstance>`, `set_library(&mut GameState,
  PlayerId, &[String]) -> Vec<CardInstance>`, `events_of_type(&[GameEvent], GameEventType) ->
  Vec<&GameEvent>` (read through serde_json, so owned events also work).
- `prompt_harness`: `board(&str) -> GameState`, `cast_now(&mut GameState, &str, PlayerId, bool)` (return
  unused), `answer_keys(&mut GameState, &[&str]) -> { events: Vec<GameEvent>, error: Option<String> }` (TS
  `{ sink, error }`; `.events` is the sink's), `open_as(&GameState, PromptKind, PlayerId) -> PendingChoice`
  (owned), `must<T>(Option<T>, &str) -> T`, `round_trip(&GameState) -> GameState`, `act(&GameState,
  ActionInput, Option<&mut Vec<Action>>) -> GameState`.
- `ky_test`: `FIXTURE_BANK` (indexable, iterable `KyTestProblem`s: a `LazyLock<Vec<_>>` or a slice),
  `book()`, `coin()`, `four()`, `gift()`, `ky_test()`, `ky_test_qd()`, `ky_two()`, `legend()` -> `CardDef`,
  `register_ky_test_fixtures()`.
- `last_boards`: `PORTAL`, `TRAP`, `FIELD_TRAP: &str`, `PORTAL_RADIANT_CARDS` (an integer), `LB_DECKS`
  (cloned with `.clone()`: a `LazyLock<(Vec<String>, Vec<String>)>`), `register_last_boards()`,
  `act(&GameState, &mut Vec<Action>, Value) -> GameState`, `portal_game(&str, Option<LastBoardInput>) ->
  { seed: String, state: GameState, log: Vec<Action> }`.
- `papaya`: `body()`, `curve()`, `curve_quickdraw()`, `field()`, `snare()`, `token()` -> `CardDef`,
  `register_papaya_fixtures()`; `prompts::register_prompt_fixtures()`.
- `catalog::vanilla_deck(i32, i32) -> Vec<String>`; `combat::{plain(), indestructible(), trampler()} ->
  CardDef`; `scripts::{HERO_POWERS (serialisable list of names), heroic_power() -> CardDef}`.

Engine (parts 2–8), shapes taken from the owners' notes where they give one:
- `subsystems::hero_power` (part 8.2): `HERO_POWERS: &[HeroPower]` (fields `name` serialising as the
  stored name, `x: i32`, `title`, `radiant_title` displayable), `HERO_POWER_NAMES` (serialisable),
  `POWER_KEY`, `POWER_RESUME: &'static str`, `STEADY_SHOT_PARAM: &str`, `hero_power` (a plain fn wrapped by
  `hook`), `roll_power() -> Effect`, `power_abilities(bool) -> Vec<ActivationDecl>`, `power_ability_of(
  &GameState, &CardInstance) -> Option<ActivationDecl>`, `power_by_name(&str) -> Option<_>`,
  `power_of(&CardInstance) -> Option<HeroPower or &HeroPower>`, `ensure_power(&mut EngineSink, &mut
  CardInstance) -> Option<_>`, `used_this_turn(&GameState, &CardInstance) -> bool`;
  `subsystems::activate::uses_this_turn(&GameState, &CardInstance) -> i32`.
- `subsystems::ky_test::easy_problem(&mut Rng) -> KyTestProblem { id, difficulty: KyTestDifficulty,
  statement, options, answer }` (String fields); `prompts::{ANSWER_KEY: &str, answer_key_of(&IndexMap<String,
  Value>) -> Option<String or &str>}`.
- `subsystems::last_boards::{freeze_last_boards (part 1's signature), last_board_candidates(&GameState,
  PlayerId, &[String]) -> Vec<LastBoardEntry>, last_board_for, rebuildable_from_id(&str, &CardDefs) ->
  bool}`; `view_for::HIDDEN_ID`; `scripts::script_of(&GameState, &str)`.
- `subsystems::papaya`: `PapayaPoint { x: i32, y: i32 }` (built by literal), `PAPAYA_LANES: i32`,
  `zone_of_point(PlayerId, PapayaPoint) -> ZoneSlot` (panics "…is not a cell of the grid"),
  `point_of_zone(PlayerId, ZoneRef) -> PapayaPoint`, `curve_at(&[PapayaPoint], i32) -> Rational { num, den }`,
  `cells_on_curve(&[PapayaPoint]) -> Vec<PapayaPoint>` (panics with TS's text), `cards_on_curve(&GameState,
  PlayerId, &[PapayaPoint], bool) -> Vec<String>` (TS's `Pick<EffectContext, "state" | "controller">` as two
  arguments).
- `subsystems::perfect_hand::{rank_perfect_hand(&GameState, PlayerId, RankPerfectHandOptions) -> Vec<Scored>,
  replace_hand_with_perfect(ReplaceHandWithPerfectArgs) -> Effect}` (both option types `Deserialize`, built
  with `json_as`; `Scored { def, score: f64, priority }`, `priority` serialising as TS's string);
  `subsystems::scorer::compare_scored(&Scored, &Scored) -> Ordering`.
- `modifiers` (part 3.2): `add_modifier(&mut EngineSink, PlayerId, ModifierExpiry, ModifierKind) ->
  PlayerModifier`, `remove_modifier`/`consume_modifier(&mut EngineSink, PlayerId, &str)`,
  `expire_modifiers(&mut EngineSink, PlayerId)`, `schedule_delayed(&mut EngineSink, PlayerId, DelayedAt,
  Resume, Option<String>, Option<i32>) -> DelayedEffect` (TS's optional `watch`, `notBefore`),
  `due_delayed(&GameState, Phase, PlayerId) -> Vec<DelayedEffect>`, `drop_delayed(&mut GameState, &str)`;
  `mana::{effective_cost(&GameState, &CardInstance, CostOptions), modifier_is_live(&GameState,
  &PlayerModifier)}`; `damage::hero_armor_of(&GameState, PlayerId) -> i32`; `params::param_value(&GameState,
  Option<&CardInstance>, &str, ParamValueOptions)`.
- `work` (part 3.1): `push_work(&mut EngineSink, Resume, Option<PlayerId>) -> WorkItem`, `owe(&mut
  EngineSink, impl Into<OweItem>)` (a `Resume` or a `WorkItem`), `park_work(&mut EngineSink, &WorkPlan,
  &PausedStep) -> WorkItem`, `WorkPlan::new(Resume, PlayerId)`, `PausedStep: Deserialize + Serialize`
  (`from: usize`), `paused_of(&IndexMap<String, Value>) -> Option<PausedStep>`, `paused(&EngineSink) ->
  bool`, `has_work`, `peek_work(&GameState) -> Option<&WorkItem>`, `take_work(&mut GameState) ->
  Option<WorkItem>`, `owed_work(&GameState, Option<&str>) -> Vec<WorkItem>`, `is_owed(&GameState, &str)`,
  `drop_work(&mut GameState, impl Fn(&WorkItem) -> bool) -> Vec<WorkItem>`, `unpark_work(&mut GameState,
  &str) -> Option<WorkItem>`, `run_next_work`/`drain_work(&mut EngineSink) -> bool`, `run_work_item(&mut
  EngineSink, &WorkItem)` (panics "no handler for owed work …"), `MAX_WORK_STEPS` from config.
- `prompts` (part 3.2): `open_prompt(&mut EngineSink, OpenPromptArgs { player, kind, aim, prompt, options,
  min, max, budget, owner, resume })`, `close_prompt(&mut EngineSink)`, `resume_at(ResumeAtArgs { def_id,
  step, hook, radiant, instance_id, data }: Default) -> Resume`, `run_resume(&mut EngineSink, &Resume,
  ResumeOptions { controller, targets: Option<Vec<Selection>>, .. }: Default)`, `run_hook_resumable(&mut
  EngineSink, &CardInstance, &str, HookResumableOptions: Default)`, `prompt_answers(&PendingChoice) -> Vec<_>`.
- `resolve`: `make_context(&mut EngineSink, Option<&CardInstance>, HookOptions { controller, targets, .. }:
  Default) -> EffectContext` (part 3.2's shape; part 25.1 and 26.1 pass `sink.reborrow()` instead),
  `apply_effects(&[Effect], &mut EffectContext)`; `triggers::{settle(&mut EngineSink, SettleOptions),
  SettleOptions: Default}`; `setup::finish_setup(&mut EngineSink)`; `state_check::state_check(&mut
  EngineSink)`.
- `zones`: `place_on_field(&mut GameState, &mut CardInstance, ZoneSlot, PlaceOnFieldOptions)` (part 2.1's
  shape), `card_at`, `slots_of(PlayerId, Row) -> Vec<ZoneSlot>`, `active_units_of -> Vec<&CardInstance>`.
- Effects, each built with `json_as` from TS's literal: `bounce`, `damage`, `effects::draw`, `steal`,
  `next_turn_mana`, `choose_mode`, `cast_new`, `effects::mana::refresh_mana`.
- `catalog::{registered_catalog, def_of(Option<&GameState>, &str) -> &CardDef, query_cost}`,
  `scripts::registered_scripts() -> IndexMap<String, CardScripts>`, testkit `register_catalog(CardDefs)`,
  `register_scripts(IndexMap<String, CardScripts>)`; `replay::{fold, FoldArgs: Default}`.

## Decisions
- Cards are clones, never live handles: wherever TS read a card object after something moved or changed it,
  the port re-reads it by id (`find_instance`), and wherever TS wrote through one (`card.memory[k] = v`,
  `card.costMod = -1`, `card.radiant = true`, `fired.faceUp = true`) it writes the state's card
  (`find_instance_mut`) and uses the refreshed copy.
- A card TS read as `zone.z === "gone"` after it ceased to exist is asserted absent from every zone
  (`find_instance(..).is_none()`), as part 25.1 did.
- Definitions are built from TS's object literals with `json_as(json!(…))` and a shallow `spread` for TS's
  `{ ...base, ...extra }`; each keeps the index TS's running `nextIndex` gave it.
- Assertions on events, views and other wire JSON read the serialised form (`serde_json::to_value`), so
  they pin TS's keys and do not depend on enum variant field names; `toMatchObject` is a local
  `matches_object` over JSON.
- Nonces: one `AtomicU32` per file (no `Mutex`, clippy.toml), as TS's module `let nonce`.
- modifiers: TS's fixtures put their delayed steps on `Script.activate` with `Resume.hook` "activate";
  SURFACE §7.2 does not port `Script.activate`, so they are `Script.delayed` with hook "delayed" (the name
  `Script::hook_named` and `effects/delay.ts`'s `DELAYED_HOOK` use for the same step). R127's
  `selfAtResume` array is a `thread_local!` `Cell<Vec<Option<String>>>` (each `#[test]` its own thread).
- pauses: TS's `registerDefaultWorkHandler(runResume …)` needs nothing (work.rs's default arm);
  `driveSequence`, `SEQUENCE_STEPS`, `sequenceResume`, `sequenceRunOf` and `hit` serve only the two tests
  that need a registered handler and are not ported (they would be dead code). "throws rather than drop
  work…" keeps its assertion with the default arm in place (an item naming no card has no step either
  way). "stops a sequence that keeps owing work" owes a fixture card's own continuation (`pz-looper`'s
  "again" step owes itself) instead of a registered handler. `unserializable` checks a serde round trip
  (a Rust state cannot hold a closure). TS `toBe` on work items is value equality.
- perfect_hand: `Number.POSITIVE_INFINITY` for Refresh's amount is `i32::MAX / 2` (no infinity in `i32`;
  half keeps `current + amount` from overflowing). `toMatch(/ph-(charger|…)/)` checks each alternative.
- last_boards: inputs TS cast past the type (`{ defId }` with no face, `null`, `{ radiant }` with no id)
  cannot be held by `LastBoardEntry`; see spec gaps.
- ky_test, papaya: TS regexes become small hand checks (no regex crate); `expect(…).toThrow(/text/)` is a
  local `panics_with` over `catch_unwind` (TS threw, the Rust functions panic with TS's message).
