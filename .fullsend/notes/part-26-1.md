# Slice: part 26, chunk 1 of 7 (engine tests 3: view, turn, setup, combat), #414 under #306
BUILDS-RUN: 0

## FILES
All ten ported whole (every `describe` as a `mod`, every `it` as a `#[test]`, in TS order, header and
rule comments kept); `#[test]` counts equal the TS `it` counts file by file (124 in all):
`crates/engine/tests/rules/{after_attack (8), animated (22), auto_end_turn (7), backrow_death (3),
backrow_piles (19), brittle (21), carried_damage (4), combat_positions (10), combat_resolution (12),
combat_validation (18)}.rs`. Notes: this file, `part-26-1.assumptions`, `spec-gaps-part-26-1.md`
(no gaps). Nothing outside these paths was written; no rebase conflicted.

## GAPS
No test was left unported (`spec-gaps-part-26-1.md` lists none).

Names called that other parts provide (the signature each call assumes; part 31 reconciles):

**Fixtures, part 24** (`crates/engine/tests/rules/fixtures/`, imported as `crate::rules::fixtures::<x>`):
- `harness`: `put(&mut GameState, def_id: &str, at: ZoneSlot, options) -> CardInstance` (the card
  as placed, owned; `options` is TS's `{ radiant? }`, passed `Default::default()` or
  `json_as(json!({ "radiant": true }))`); `sink_for(&mut GameState) -> EngineSink<'_>` (TS
  `sinkFor(state)`: the sink must own its events and rng somehow, e.g. leaked boxes, since
  `EngineSink` borrows both; tests read `sink.state`, `sink.events`, `sink.rng.cursor()`);
  `slot(PlayerId, Row, i32) -> ZoneSlot`; `in_hand(&mut GameState, &str, PlayerId, Option<i32>) ->
  Vec<CardInstance>`; `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`;
  `events_of_type(&[GameEvent], GameEventType) -> Vec<&GameEvent>` (used through `.len()`,
  `.is_empty()` and `serde_json::to_value`); `set_library(&mut GameState, PlayerId, &[String]) ->
  Vec<CardInstance>`.
- `damage_combat`: `playing(&str) -> GameState`, `notes(&GameState) -> Vec<String>`,
  `recorder(GameState) -> Recorder` with `start: GameState`, `log: Vec<Action>`, `state(&self) ->
  &GameState`, `play(&mut self, ActionInput) -> ReduceResult`; `replays_to(&GameState, &[Action],
  &GameState) -> bool`, `round_trip(&GameState) -> GameState`, `answer(&GameState) -> ReduceResult`;
  card defs as functions returning `CardDef`: `veteran`, `grunt`, `wall`, `pawn`, `veteran_asker`,
  `cleave_veteran`, `turncoat`, `reborn_veteran`, `death_asker`, `bauble`, `rattle`, `storm`,
  `voidwalker`.
- `field`: `playing`, `act(&GameState, ActionInput) -> GameState`, `act_result(&GameState,
  ActionInput) -> ReduceResult`, `flush(&mut GameState, PlayerId, Option<i32>)`,
  `notes_of(Option<&CardInstance>) -> Vec<String>`; card fns `tesla`, `springer`, `asker`,
  `spatula`, `golem`, `wisp`, `tower`, `fuser`, `cover`, `banner`, `watcher`, `listener`, `ears`,
  `wrecker`, `mourner`.
- `combat`: card fns `plain`, `stacker`, `indestructible`, `taunter`, `armoured`, `big_dfender`,
  `deft_duelist`, `spikey_pillow`, `big_body`, `first_striker`, `moths`, `charger`, `pacifist`,
  `rusher`, `zero_attack`.
- `catalog`: `spell_def(i32, Value /* Partial<CardDef> */) -> CardDef`, `vanilla_deck(Option<i32>,
  Option<i32>) -> Vec<String>`. `scripts`: `heroic_power()`. `instance_data`: `asker`,
  `brittle_trap`, `brittle_unit`, `instance_game(Option<&str>, Option<(Vec<String>, Vec<String>)>) ->
  GameState`.

**Engine** (through `jackioh_engine::testkit::*` unless a path is given):
- `resolve` (part 3): `HookOptions { controller: Option<PlayerId>, targets, modes, data }` with
  `Default`; `make_context(EngineSink<'a>, Option<&CardInstance>, HookOptions) -> EffectContext<'a>`
  (called with `sink.reborrow()` or `sink_for(..)`); `apply_effects(&[Effect], &mut EffectContext)`;
  `HookName` enum (`HookName::EndOfTurn`).
- `triggers`: `settle(&mut EngineSink)`, `run_hooks_in_trigger_order(&mut EngineSink, HookName,
  Option<PlayerId>)`. `state_check`: `state_check(&mut EngineSink)`, `sacrifice_now(&mut EngineSink,
  &CardInstance)`. `work`: `owed_work(&GameState, Option<&str>) -> Vec<WorkItem>`.
- `combat`: `AFTER_ATTACK_WORK: &str`; `AttackTarget` (= `damage::DamageTarget`) an enum
  `Unit { instance: CardInstance } | Hero { player: PlayerId }` with `PartialEq, Debug`;
  `attack_targets(&GameState, &CardInstance) -> Vec<AttackTarget>`; `can_attack(&GameState,
  &CardInstance, &AttackTarget) -> bool`; `why_cannot_attack(..) -> Result<(), EngineError>`;
  `has_exertion(&GameState, &CardInstance, ExertionKind) -> bool` (`ExertionKind::{Attack, Switch}`);
  `is_sick(&GameState, &CardInstance)`; `switch_position(&mut EngineSink, &CardInstance,
  SwitchPositionOptions { spend_exertion: Option<bool>, to: Option<Position> }) -> Result<(),
  EngineError>`; `declare_attack(&mut EngineSink, &CardInstance, &AttackTarget) -> Result<(),
  EngineError>`; `force_attack(&mut EngineSink, &CardInstance, &AttackTarget)`;
  `force_attacks_on(&mut EngineSink, &[CardInstance], &AttackTarget, Option<u32>)`;
  `random_attack_targets(&GameState, &CardInstance, among) -> Vec<AttackTarget>` (`among` built with
  `json_as(json!("enemyUnits"))`, so any serde enum fits).
- `damage`: `deal_damage(&mut EngineSink, DamageArgs { source: Option<CardInstance>, target:
  DamageTarget, amount: i32, flags: Option<_> }) -> i32`. `layers`: `unit_view(&GameState,
  &CardInstance) -> UnitView`.
- `animated` (part 2): `animate_at_turn_start(&mut EngineSink, PlayerId)`, `animate_card(&mut
  EngineSink, &CardInstance, options /* { position? } */) -> bool`, `animated_kind_of(&GameState,
  &CardInstance) -> Option<_>`, `face_type_of(&GameState, &CardInstance) -> CardType`,
  `is_animated`, `return_at_cleanup(&mut EngineSink, PlayerId)`, `return_home(&mut EngineSink,
  &CardInstance) -> bool`. `faces::card_type_of(&GameState, &CardInstance) -> CardType`.
  `traps::is_spent(&GameState, &CardInstance) -> bool`.
- `zones` (part 2): `ZoneSlot` passed by value; `card_at`, `carried_at(&GameState, ZoneSlot) ->
  Option<&CardInstance>`; `beneath_at(&GameState, ZoneSlot)` (a list of `CardInstance`);
  `home_of(&GameState, &str) -> Option<&HomeZone>`; `is_reserved(&GameState, ZoneSlot)`;
  `lock_zone`, `reserve_zone(&mut GameState, ZoneSlot)`; `active_units_of(&GameState, PlayerId)` (a
  list of cards); `is_buried`, `is_carried(&GameState, &CardInstance)`; `stacked_onto(&CardInstance)
  -> Option<String or &str>`; `place_on_field(&mut GameState, &mut CardInstance, ZoneSlot, options /*
  { stack? } */) -> bool`; `remove_from_field(&mut GameState, &CardInstance, options /* { withPile? }
  */) -> bool`.
- `play_choices`: `legal_zones_for(&GameState, PlayerId, &CardInstance, Option<_ /* tributes */>) ->
  Vec<ZoneChoice>`, `plays_on_stack(&GameState, &CardInstance) -> bool`.
  `jackioh_engine::subsystems::activate::why_cannot_activate_ability(&GameState, PlayerId, &str,
  Option<&str>) -> Result<(), EngineError>`.
- `brittle` / `brittle_count` (part 2): `brittle_tick(&mut EngineSink, PlayerId)`,
  `active_brittle_count(&CardInstance) -> Option<i32>`, `give_brittle_count(&GameState, &mut
  CardInstance, i32)`, `gain_brittle_count(&GameState, &mut CardInstance, i32)`.
- `effects` (parts 6–7), every argument type `Deserialize` (built with `json_as(json!(…))`, SURFACE
  §6.6) and `Default` where TS defaulted it to `{}`: `effects::forced_attacks_on`,
  `effects::{bounce, exile}`, `effects::combat::forced_attack_random`, `effects::damage::damage_all`,
  `effects::steal::steal`, `effects::destroy::destroy_all`, `effects::move_::bounce_card(&mut
  EngineSink, &CardInstance)`, `effects::swap::swap_board()`, `effects::transform::{transform,
  vanilla}`, `effects::reveal::reveal`, `effects::targets::cards_in_scope(&EffectContext,
  &BoardScope) -> Vec<CardInstance>`.
- `view_for`: `HIDDEN_ID: &str`. `replay` (part 5): `FoldArgs` with `Default` (`seed`, `decks`,
  `log`, the rest optional), `fold(&FoldArgs) -> FoldResult { state, errors }`, `hash_state`.
- `catalog::registered_catalog()`, `scripts::registered_scripts()` (owned or `&'static`; the test
  `.clone()`s it), `CardScripts: Clone`, the testkit's one-argument `register_catalog(CardDefs)` and
  `register_scripts(IndexMap<String, CardScripts>)` (SURFACE §8).

## Decisions
- **Live objects.** TS tests hold live `CardInstance` references and write through them. Here a test
  holds an owned copy for its id, reads the card back with `find_instance` after every engine call
  (local `by_id`/`live`), and writes with `find_instance_mut` (`by_id_mut`). Every assertion TS made
  on a live object is made on the card as it stands in the state.
- **Sinks.** `sinkFor(state)` is `sink_for(&mut state)`; while the sink lives the state is read as
  `sink.state`. `sinkFor(state, events)` (an outside events array) is a local `run(state, |sink| …)`
  that hands back `sink.events.clone()`. A card handed to a sink-taking function is cloned out of
  `sink.state` first (no `&mut sink` and `&sink.state` in one call).
- **TS default and optional parameters**: a scalar one (`count = 1`, `mana = 10`, `seed = "…"`,
  `decks?`, `since = …`, `ability?`, `tributes = []`) is a trailing `Option<T>` and an omitted one is
  `None`; an options object (`options = {}`) is a struct, omitted as `Default::default()`, given as
  `json_as(json!({ … }))` when TS's type is anonymous (`{ stack? }`, `{ radiant? }`, `{ withPile? }`,
  `{ position? }`) and as a struct literal when it has a TS name (`HookOptions`,
  `SwitchPositionOptions`). `Partial<CardDef>` overrides are a `Value`.
- **Refusal strings** (`whyCannotAttack`, `whyCannotActivateAbility`, `switchPosition`'s and
  `declareAttack`'s `{ error? }`) are `Result<(), EngineError>` (SURFACE §4.4.9); each file's
  `refusal()` turns one into TS's `string | null` for the assertion. `ReduceResult.error` is
  SURFACE's `Option<String>`. Error-text assertions keep TS's text; `toMatch(/x/)` is `contains`.
- **Literals.** Actions are `json_as::<ActionInput>(json!({ … }))` (the nonce added by the fixture or
  by `with_nonce`); effect arguments `json_as(json!({ … }))`; anonymous string-union arguments
  (`randomAttackTargets`' `among`) `json_as(json!("…"))`.
- **Expectations.** `toEqual` on events, views and homes compares their serde JSON with `json!`
  (absent `Option`s are absent keys, as TS's `undefined`); `toMatchObject` is a small local
  `matches_object` over `Value`s (subset on objects, element-wise and same length on arrays);
  `not.toHaveProperty("autoEndTurn")` checks the key is absent from the serialised value.
  `toBeUndefined` on an optional field is `None`; `toBe(true)` on `faceUp` is `Some(true)`.
- **Module `let`s** (`nonce`, `counter`, `priced`) are `static AtomicU32`s.
- **Names**: an `R<n>` anywhere in a title is a leading `r<n>_` token, several in title order
  (`(R212)` at the end of an R447 title gives `r447_r212_…`); a describe's `§4.5` is `section_4_5_…`
  at the front or `…_4_5` inside; punctuation is dropped. Two TS files have `it`s outside any
  `describe` (combat-resolution l.154 and l.172, combat-validation l.310): they stay top-level
  `#[test]`s between the `mod`s, in TS order.
- **Local helpers kept to their callers**: combat-validation's `attackVia(…, player = "p1")` takes no
  player (no caller passes one); brittle's `tickAt(…, doSettle = true)` always settles (no caller
  passes false) and `at(turn, active = "p1", seed = "brittle")` always uses "brittle" (no caller
  passes a seed). No assertion changes with it.
- **Exposure, recorded honestly**: one `grep` meant for part 1's frozen files also printed ten lines
  of `crates/engine/src/{brittle,carriers,reduce}.rs` (a `crate::resolve::HookOptions {` literal, a
  private `carried_at(&ZoneSlot)` in carriers, `AttackTarget::Unit { instance, .. }` /
  `Hero { player, .. }` patterns and `pub struct ReduceResult`). Nothing else under `crates/*/src/`
  was opened. The tests do not lean on it: `HookOptions`, `AttackTarget` and `ReduceResult` are TS's
  and SURFACE's names, `ZoneSlot` stays by value as decided before, and `..` in the target patterns
  is the form that compiles either way.
