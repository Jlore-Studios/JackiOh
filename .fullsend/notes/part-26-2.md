# Slice: part 26, chunk 2 of 7 (engine tests 3: view, turn, setup, combat), #414 under #306
BUILDS-RUN: 0

## FILES
All ten ported whole (every `describe` a `mod`, every `it` a `#[test]`, in TS order, the header and
every rule- or ruling-citing comment kept); `#[test]` counts equal the TS `it` counts file by file
except `condition_active` (99 in all):
`crates/engine/tests/rules/{combat_property (6), condition_active (29 of 33), config (5),
control_change_property (4), control_change (15), damage_pipeline (8), damage (17), destroyed_face (2),
endgame (12), faces (7)}.rs`. `condition_active` drops the brief's l.770–825 (the B10 `describe`, its
three `it`s, with its `textOf`/`specBetween` helpers from l.765), and one `it` has no Rust form (see
GAPS). Notes: this file, `part-26-2.assumptions`, `spec-gaps-part-26-2.md`. Nothing outside these
paths was written; no rebase conflicted.

## GAPS
Tests not ported (also in `spec-gaps-part-26-2.md`):
- `conditionActive.test.ts:256` "R195 B1: only an answer of exactly true lights the card": the hook
  answers a truthy non-boolean (`1`, `"yes"`). Part 1's `ConditionHook` returns `bool`.
- `conditionActive.test.ts:770–825` (the B10 block): dropped by the brief (#133).

Names called that other parts provide (the shape each call assumes; part 31 reconciles):

**Fixtures, part 24** (`crate::rules::fixtures::<x>`), aligned with part 24.6's and 24.7's notes:
- `harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState` (TS's default seed
  written out, `"engine-test"`); `put(&mut GameState, &str, ZoneSlot, Value) -> CardInstance` (options
  `json!({})` or `json!({ "radiant": … })`; the card as placed, owned); `slot(PlayerId, Row, i32) ->
  ZoneSlot`; `in_hand(&mut GameState, &str, PlayerId, i32) -> Vec<CardInstance>`;
  `events_of_type(&[GameEvent], GameEventType)` → a `Vec` of `GameEvent` or `&GameEvent` (read only
  through `.iter()`, `.len()`, `.is_empty()`, `[i]` and serde, so either compiles).
- `combat`: the `LazyLock<CardDef>` statics `plain`, `big_body`, `zero_attack`, `taunter`, `rusher`,
  `charger`, `first_striker`, `shielded`, `armoured`, `indestructible`, `poisonous`, `lifestealer`,
  `trampler`, `trample_lifesteal`, `cleaver`, `pacifist`, `stacker`, `moths`, `big_dfender`,
  `deft_duelist`, `spikey_pillow` (read `x.id`, `x.clone()`).
- `damage_combat`: statics `anime_armor`, `argus`, `bolt`, `grunt`, `hidden_argus`, `hidden_lens`,
  `lance`, `lens`, `solar`, `sweep`, `wall`; `playing(&str) -> GameState`; `recorder(&GameState) ->
  Recorder` with `start: GameState`, `log: Vec<Action>`, `state(&self) -> &GameState`, `play(&mut self,
  impl Serialize) -> ReduceResult` (handed a `json!` literal); `replays_to(&GameState, &[Action],
  &GameState) -> bool`.
- `field::playing(&str) -> GameState`. `instance_data`: statics `billy`, `blood_moon`;
  `instance_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`.
- `catalog`: `spell_def(i32, Value) -> CardDef`, `token_def(&str, [Tag::Token])` (24.6's own call
  form), `vanilla_deck(i32, i32) -> Vec<String>`. `scripts` (24.7, functions): `double_edge()`,
  `infinite_reserves()`, `anti_oneshot()`.

**Engine** (through `jackioh_engine::testkit::*` unless a path is given):
- SURFACE §6.1: `reduce`, `begin_game`, `legal_actions`, `view_for`, `mulligan_owed(&GameState) ->
  Vec<PlayerId>`; `ReduceResult { state, events, error: Option<String> }`.
- `combat` (part 2): `AttackTarget` (= `DamageTarget`) `Unit { instance: CardInstance } | Hero { player }`;
  `can_attack(&GameState, &CardInstance, &AttackTarget) -> bool`; `declare_attack(&mut EngineSink,
  &CardInstance, &AttackTarget)` (result taken with `let _ =`; `Result<(), EngineError>` by SURFACE
  §4.4.9); `force_attack` and `resolve_combat(&mut EngineSink, &CardInstance, &AttackTarget)`;
  `is_active_on_field`, `is_sick(&GameState, &CardInstance) -> bool`; `why_cannot_attack(&GameState,
  &CardInstance, &AttackTarget) -> Result<(), EngineError>` (TS `string | null`, read through a local
  `refusal`); `attack_targets(&GameState, &CardInstance) -> Vec<AttackTarget>`.
- `state_check::state_check(&mut EngineSink)`; `triggers::settle(&mut EngineSink, SettleOptions:
  Default)` (TS `options = {}`).
- `layers`: `unit_view(&GameState, &CardInstance) -> UnitView { attack, max_health, health, keywords:
  Vec<Keyword>, armor, position: Position }`; `unit_has(&GameState, &CardInstance, KeywordKind) -> bool`.
- `zones`: `ZoneSlot { player, row, lane }` by value; `active_units_of`, `dormant_units_of(&GameState,
  PlayerId)` (a list of cards, read through `.iter()`/`.first()` and `.id`, owned or borrowed);
  `card_at(&GameState, ZoneSlot) -> Option<&CardInstance>`; `place_on_field(&mut GameState,
  CardInstance, ZoneSlot, options) -> bool` with `{ stack }` built by `json_as`; `lands_face_down(
  &GameState, &CardInstance, Row) -> bool`.
- `damage`: `deal_damage(&mut EngineSink, DamageArgs { source: Option<CardInstance>, target:
  DamageTarget, amount: i32, flags: Option<_: Deserialize> }) -> i32`; `heal_unit(&mut EngineSink,
  &CardInstance, i32)`, `heal_to_full(&mut EngineSink, &CardInstance)`, `heal_hero` and
  `heal_hero_up_to(&mut EngineSink, PlayerId, i32)`, `lose_health(&mut EngineSink, PlayerId, i32)`, all
  `-> i32`; `hero_damage_cap(&GameState, PlayerId) -> Option<i32>`; `hero_damage_divisor(&GameState,
  PlayerId) -> i32`; `hero_hit_amount(&GameState, PlayerId, i32, bool) -> i32` (TS `pierce = false`
  passed explicitly); `spell_damage_of(&GameState, PlayerId) -> i32`.
- `draw::draw_one(&mut EngineSink, PlayerId, Option<_>)` (TS `link?`).
  `jackioh_engine::subsystems::lethal::projected_hero_damage(&GameState, PlayerId, i32) -> i32`.
- `resolve`: `make_context(EngineSink, Option<CardInstance>, HookOptions) -> EffectContext` (called
  with a fresh sink or `sink.reborrow()`), `HookOptions { controller: Option<PlayerId>, .. }: Default`;
  `apply_effects(&[Effect], &mut EffectContext)`. An `Effect` is run as `(effect.apply)(&mut ctx)`.
- `faces`: `running_face(&GameState, &CardInstance)` (a `CardFace`, owned or borrowed; `.type_` read),
  `card_type_of(&GameState, &CardInstance) -> CardType`. `traps`: `is_trap_type`, `is_field_trap(
  &GameState, &CardInstance) -> bool`, `consume_trap(&mut EngineSink, &CardInstance)`.
  `play_choices::row_for_card(&GameState, &CardInstance) -> Row`. `catalog::query(<CatalogQueryArgs:
  Deserialize>) -> Vec<CardDef>`.
- `condition::condition_active(&GameState, &CardInstance, PlayerId, ConditionZone) -> bool`;
  `prompts::open_prompt(&mut EngineSink, OpenPromptArgs { player, kind, aim: None, prompt: String,
  options: Vec<PromptOption>, min: None, max: None, budget: None, owner: None, resume: Resume })`.
- `jackioh_engine::subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs
  { ingredients: Vec<CardInstance>, target: Option<CardInstance>, to_hand: Option<PlayerId>, .. }:
  Default}`. `jackioh_engine::subsystems::rotation::{rotate_rings(&mut EngineSink, RotationArgs),
  RotationArgs { direction: RotationDirection, perspective: PlayerId, radiant: Option<bool> }}`.
- `jackioh_engine::effects` (argument structs `Deserialize`, built with `json_as(json!(TS literal))`):
  `damage`, `summon`, `plague`, `steal`, `rotate`; `steal_all(Default::default())`, `swap_board()`;
  `cards_in_card_scope(&EffectContext, &CardScope, options: Default) -> Vec<ScopedCard>` (`.card.id`).
- `catalog::registered_catalog()` and `scripts::registered_scripts()` (owned or borrowed; `.clone()`d,
  merged, handed to the testkit's `register_catalog(CardDefs)` / `register_scripts(IndexMap<String,
  CardScripts>)`), answering the thread-local override (SURFACE §8).

## Decisions
- **Live objects.** A TS test holds the live `CardInstance` and writes through it; here a test keeps
  the owned copy for its id, reads the card back with `find_instance` before every assertion and every
  engine call that takes it (local `live`), and writes with `find_instance_mut`. Every TS assertion on
  a live object is made on the card as the state holds it. TS's `toBe(spell)` on a hook's context is
  value equality with the live card; `toBe(state)` is the state's address.
- **Sinks.** Built locally from the frozen types (`Rng::new(&state.seed, state.rng_cursor)` +
  `EngineSink::new`, or a local `with_sink(state, |sink| …)`), as part 24.1 decided; the fixtures'
  leaking `sink_for` is not used. Where TS wrote the cursor back (`run`, `rotate`, `apply`), so does
  the port.
- **Names.** Every `R<n>` in a title is a leading `r<n>_` token, several in title order
  (`r171_r53_…`, `r2_r389_…`, `r69_m2_gate_…`), as part 26.1 did; `§4.4` is `s4_4`, `#86` is `c86`,
  punctuation is `_`. A `describe` naming rulings keeps them in its `mod` (`r18_r19_heal_and_lose_…`).
- **Literals.** Actions are `json_as(json!(TS literal))` (`ActionInput` + `with_nonce`, or a whole
  `Action`); CardDefs and other TS object literals likewise; `toEqual` on events and views compares
  serde JSON; `toMatchObject` is a local `matches_object`; `"x" in obj` checks the serialised key.
- **Module `let` counters** (`nonce`, `seq`) are `static AtomicU32`s (uniqueness is all a nonce needs).
  TS `act` in endgame answered `{ state, events: [] }`; the empty list is never read, so it answers the
  state.
- **fast-check** (control-change.property) has no Rust counterpart in the workspace: the cases are drawn
  from the engine's `Rng`, one stream per run (`"20260922:<run>"`), with the arbitraries' shapes and
  `fc.option`'s one-in-five nil kept, and TS's run counts. A failure names its run index.
- **`vi.fn` mocks** (conditionActive) are per-test `ConditionHook`s answering an `Arc<AtomicBool>` and
  sending each context down an `mpsc` channel (no `Mutex`/`RefCell`, clippy.toml); a `Cell<Vec<_>>`
  keeps what was read. A fresh set per `game()` is TS's `beforeEach`; the thread-local registries make
  `afterAll`'s restore unneeded. TS's `nextIndex` (1950) is written out per def (1951–1962).
- **`BASE`** (control-change.property's module constant) is built once per test on the test's thread,
  since the fixture registries it needs are thread-local.
- A local binding that would shadow an imported fixture static is renamed (`is_indestructible_now`),
  as part 24.6's notes warn (E0530).
- **Exposure, recorded honestly**: one `grep` over `crates/engine/tests/rules/*.rs` meant for my own
  files also printed single call lines (`instance_game(…)`, `token_def(…)`) of other parts' test files
  (brittle, effects_*, instance_data, params, state, statecheck, zones). Nothing under `crates/*/src/`
  other than part 1's frozen files was opened; my calls follow the fixture authors' notes, not those
  lines.
