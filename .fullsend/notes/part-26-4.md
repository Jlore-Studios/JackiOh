# Slice: part 26, chunk 4 of 7 (engine tests 3: view, turn, setup, combat), #414 under #306
BUILDS-RUN: 0

## FILES
All eleven ported whole (every `describe` as a `mod`, every `it` as a `#[test]`, in TS order, header and
rule comments kept); `#[test]` counts equal the TS `it` counts file by file (112 in all):
`crates/engine/tests/rules/{lethal (8), library_copies (6), mulligan_concurrent (10), own_library (8),
params (10), pools (9), preview_ids (2), preview (20), query (18), recruit_variants (11), reduce (10)}.rs`.
Notes: this file, `part-26-4.assumptions`, `spec-gaps-part-26-4.md` (no test left unported; the partial
assertions are listed there). Nothing outside these paths was written; no rebase conflicted.

## GAPS
No test was left unported (`spec-gaps-part-26-4.md`).

Names called that other parts provide (the signature each call assumes; part 31 reconciles).

**Fixtures, part 24** (`crate::rules::fixtures::<x>`), used as staging's fixture files define them at
`2d091b1`:
- `harness`: `new_game(&str, Option<(Vec<String>, Vec<String>)>)`, `setup_catalog()`, `put(&mut GameState,
  &str, ZoneSlot, Value) -> CardInstance`, `slot`, `in_hand(&mut GameState, &str, PlayerId, i32) ->
  Vec<CardInstance>`, `set_library(&mut GameState, PlayerId, &[impl AsRef<str>]) -> Vec<CardInstance>`,
  `sink_for(&mut GameState) -> EngineSink<'_>`, `events_of_type(&[GameEvent], GameEventType) ->
  Vec<GameEvent>`.
- `catalog`: `vanilla_deck(i32, i32)`, `unit_def(i32, Value)`, `spell_def(i32, Value)`.
- `combat` (statics): `plain`, `big_body`, `armoured`, `indestructible`, `shielded`, `trampler`.
- `scripts` (fns): `anti_oneshot()`, `hinder()`, `cn_virus()`, `stockpile()`.
- `instance_data`: statics `body`, `nerfer`, `numbered`, `radiant_number`; `instance_game(&str, Option<…>)`.
- `generation`: `Run { start, log, state }`, `playing`, `frozen`, `act(&Run, impl Serialize)`,
  `answer(&Run, Selection, Option<PlayerId>)`, `replayed`, `hand_card(&mut GameState, &str, PlayerId)`,
  `asker_answers()`, `clear_asker_answers()`; statics `asker`, `body`, `cheap_unit`, `deck_spell`,
  `field_trap`, `pile_on`, `plain_trap`, `pricy_unit`, `x_unit`.

**Engine** (through `jackioh_engine::testkit::*` unless a path is given):
- `subsystems::lethal::{projected_damage(&GameState, &CardInstance, &AttackTarget) -> i32, is_lethal(..) ->
  bool, defending_hero(&AttackTarget) -> PlayerId}`; `combat::AttackTarget::{Unit { instance:
  CardInstance }, Hero { player }}`; `layers::unit_view(&GameState, &CardInstance) -> UnitView`.
- `resolve`: `make_context(EngineSink<'a>, Option<CardInstance>, HookOptions) -> EffectContext<'a>`,
  `HookOptions { controller: Option<PlayerId>, .. }: Default`.
- `effects` verbs, each taking one `Deserialize` argument built with `json_as(json!(TS literal))`:
  `add_library_copies`, `add_to_hand`, `discard_random`, `radiant_chance`, `set_radiant`, `shuffle_into`,
  `transform`, `recruit`, `recruit_all`, `forced_attacks`; `swap_library()` takes none (TS's call has none).
- `effects::tune::TuneDirection::{Upgrade, Degrade}` as `params::steppable_params`' `change` (TS's inline
  `"upgrade" | "degrade"`, the same literals as tune.ts's `TuneDirection`).
- `params` (part 2–4): `param(&EffectContext, &str) -> i32` (panics with TS's message on a missing card,
  an undeclared key or a bad part path); `param_value(&GameState, Option<&CardInstance>, &str,
  ParamValueOptions) -> i32` with `ParamValueOptions { def_id: Option<String>, radiant: Option<bool> }:
  Default`; `param_step(&Param, i32) -> i32`; `params_of(&GameState, &str)` (iterable of `Param`);
  `params_view(&GameState, &CardInstance) -> Option<IndexMap<String, i32>>`; `step_param(&mut
  CardInstance, &str, i32)`; `set_param(&mut CardInstance, &str, i32)`; `steppable_params(&GameState,
  &CardInstance, TuneDirection) -> Vec<_>` whose items have `.delta: i32`.
- `numbers::numbers_on(&GameState, &CardInstance) -> Vec<NumberOnCard>` with `.id: String`, `.value: i32`.
- `work::PART_KEY: &str`.
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs { ingredients:
  Vec<CardInstance>, target: Option<CardInstance>, to_hand: Option<PlayerId>, .. }: Default}`.
- `preview::preview_of(&GameState, &CardInstance, PlayerId, ConditionZone) -> Option<Vec<PreviewValue>>`.
- `prompts::open_prompt(&mut EngineSink, OpenPromptArgs)` with `OpenPromptArgs: Deserialize` (TS literal).
- `zones`: `place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot, options: Deserialize) -> bool`
  (the harness's form), `move_to_zone(&mut GameState, &CardInstance, OffFieldZone, Default options)`,
  `OffFieldZone::{Hand, Library, Graveyard, Exile}` (TS `OffFieldZone`, also `zone_cards`' argument).
- `query`: `hero_of(&GameState, PlayerId) -> HeroView` (owned, pub `health`/`armor`), `zone_cards(&GameState,
  PlayerId, OffFieldZone) -> Vec<CardInstance>`, `zone_count(..)`, `cards_played_this_turn(&GameState,
  PlayerId) -> i32`, `played_ids_this_turn(..) -> Vec<String>`, `was_played_this_turn(&GameState, PlayerId,
  CardOrId) -> bool`, `played_earlier(&GameState, PlayerId, Option<CardOrId>) -> i32`,
  `killer_of(&GameState, Option<&CardInstance>) -> Option<CardInstance>` (or `&`), `max_mana_of(..) -> i32`,
  and **`query::CardOrId<'a> { Card(&'a CardInstance), Id(&'a str) }`** for TS's `CardInstance | string`
  (a name these files coin; the generation fixture has a test-side trait of the same name, not imported here).
- `damage`: `lose_health(&mut EngineSink, PlayerId, i32)`, `deal_damage(&mut EngineSink, DamageArgs { source:
  Option<CardInstance>, target: DamageTarget, amount, flags: None })`. `draw::draw_one(&mut EngineSink,
  PlayerId, None)`. `turn::start_turn(&mut EngineSink, PlayerId)`. `state_check::state_check(&mut EngineSink)`.
- `setup`: `mulligan_owed(&GameState) -> Vec<PlayerId>`, `mulligan_prompt_for(&GameState, PlayerId) ->
  Option<_>` with `.options[].key`. `replay`: `fold(&FoldArgs) -> FoldResult { state, errors }`,
  `FoldArgs: Default`, `hash_state`. `reduce`: `begin_game`, `reduce`, `legal_actions`, `seat_to_act ->
  Option<PlayerId>` (a live game's `None` falls back to `state.active`, as TS's last fallback).
- `catalog`: `registered_catalog()`, `def_of(&GameState, &str)` (owned or `&`), `def_by_index(SetName, &str)
  -> Option<_>`, `query(&CatalogQueryArgs) -> Vec<CardDef>` (or `Vec<&CardDef>`), **`CatalogQueryArgs`**
  (TS's `CatalogQuery & { defId?, token? }`, `Serialize + Deserialize`), `excluding_def_id(&CatalogQueryArgs,
  Option<&str>) -> CatalogQueryArgs`, `self_def_ids(&str) -> Vec<String>`, `fused_id_parts(&str) ->
  Option<Vec<String>>`. `scripts::registered_scripts()` (cloned to an `IndexMap<String, CardScripts>`).
- `own_library::own_library_view(&GameState, PlayerId) -> LibraryView`; `view_for::HIDDEN_ID: &str`.

## Decisions
- Fixture definitions are named as staging's fixture files define them (combat, instance_data and
  generation as `LazyLock` statics read `plain.id`; scripts as functions read `hinder().id`). Test-local TS
  `const` definitions are functions (`cheap()`, `pv_unit()`); TS's `nextIndex` counter is written out.
- TS held live objects; every card is re-read from the state (`find_instance`) before a call and written
  through `find_instance_mut` where TS assigned to it.
- Ruling tokens lead every name, in title order, where the title cites them anywhere (`r265_r216_…`,
  `r361_r13_r78_…`); `§x.y` is `sx_y`, `#n` is `cn`; a `describe` without one is its title snake-cased.
- `vi.fn` hooks (preview) and the module `let answered` (preview-ids) are thread-local `Cell<Vec<_>>` logs
  (no `RefCell`/`Mutex`, both banned by clippy.toml); TS module counters (`nonce`, `seq`) are `AtomicU32`s.
- Object identity (`toBe` on objects) is equality; `toMatchObject` is a local matcher over
  `serde_json::Value`; `toMatch(/re/)` on an error is `contains` of the literal part.
- A TS `throw` reached by a test (`param`) is a panic caught with `catch_unwind` and matched on its text.
