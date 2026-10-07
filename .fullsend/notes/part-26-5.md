# Slice: part 26, chunk 5 of 7 (engine tests 3: view, turn, setup, combat), #414 under #306
BUILDS-RUN: 0

## FILES
All twelve ported whole (every `describe` as a `mod`, every `it` as a `#[test]`, in TS order, header and
rule comments kept); `#[test]` counts equal the TS `it` counts file by file (108 in all):
`crates/engine/tests/rules/{replacements (28), replay_scripted (3), replay (2), restrictions (10), rng (7),
rotation (12), rounds (4), self_tribute (4), setup_aside (17), setup (11), shuffle_random (3), state (7)}.rs`.
Notes: this file, `part-26-5.assumptions`, `spec-gaps-part-26-5.md`. Nothing outside these paths was
written; no rebase conflicted.

## GAPS
Tests not fully expressible: two, both listed in `spec-gaps-part-26-5.md` (restrictions' `registerAttackBar`
block, which SURFACE §6.6 does not port; rng's second process). No `it` was dropped.

Names called that other parts provide (the signature each call assumes; part 31 reconciles):

**Fixtures, part 24** (`crates/engine/tests/rules/fixtures/`, imported as `crate::rules::fixtures::<x>`):
- `harness`: `setup_catalog()`; `new_game(&str, Option<(Vec<String>, Vec<String>)>) -> GameState`;
  `put(&mut GameState, &str, ZoneSlot, options /* { radiant? }: Default + Deserialize */) -> CardInstance`;
  `slot(PlayerId, Row, i32) -> ZoneSlot`; `in_hand(&mut GameState, &str, PlayerId, Option<i32>) ->
  Vec<CardInstance>`; `set_library(&mut GameState, PlayerId, &[String]) -> Vec<CardInstance>`;
  `events_of_type(&[GameEvent], GameEventType)` (a `Vec` of `&GameEvent` or `GameEvent`, read only through
  `serde_json::to_value`); `play_random_game(&str, Option<(Vec<String>, Vec<String>)>)` returning a struct
  with `state: GameState`, `log: Vec<Action>`, `decks: (Vec<String>, Vec<String>)`. No `sink_for` is
  called (Decisions).
- `catalog`: `vanilla_catalog(Option<i32>, Option<i32>) -> CardDefs`, `vanilla_deck(Option<i32>,
  Option<i32>) -> Vec<String>`, `token_def(&str, Option<Vec<Tag>>) -> CardDef`, `unit_def(i32, Value /*
  overrides */) -> CardDef`.
- `scripts`: `going_long()`, `heroic_power()`, `hinder()`, `anti_oneshot()` (each `-> CardDef`);
  `HERO_POWERS: &[&str]`.
- `prompt_harness`: `cast_now(&mut GameState, &str, Option<PlayerId>, Option<bool>) -> Vec<GameEvent>` —
  TS returned the sink; the tests read only its `events`, and a Rust sink cannot outlive the state it
  borrows. It writes the rng cursor back as TS did.
- `rng_child`: `run(args: &[&str]) -> String` — the script's argv (`seed`, `cursor`, `count`) in, the JSON
  it printed (`{ "draws": [...], "cursor": n }`) out.
- `damage_combat`: `playing(&str) -> GameState`; `notes(&GameState) -> Vec<String>`; `note(&str) ->
  Effect`; `ask_controller(&str) -> Effect`; `recorder(GameState) -> Recorder` with fields `start:
  GameState`, `log: Vec<Action>` and methods `state(&self) -> &GameState`, `play(&mut self, ActionInput) ->
  ReduceResult` (the `.clone()` after `state()` also fits an owned return); `replays_to(&GameState,
  &[Action], &GameState) -> bool`; `round_trip(&GameState) -> GameState`; `answer(&GameState) ->
  ReduceResult`; `event_types(&[GameEvent]) -> Vec<String>`; `in_pile(&GameState, PlayerId, OffFieldZone,
  &str) -> bool`; card defs as zero-argument fns returning `CardDef`: `anime_armor`, `argus`, `blood_moon`,
  `bolt`, `charger`, `echo_gambit`, `fighter`, `gambit`, `gambit_asker`, `grunt`, `joro`, `leech`, `mend`,
  `phoenix`, `pile_on`, `rattle`, `second_wind`, `shadowstep`, `statue`, `storm`, `top_loser`,
  `vital_kill`, `voidwalker`, `wall`, `warded`, `watcher`.

**Engine** (through `jackioh_engine::testkit::*` unless a path is given):
- `reduce`/`replay` (part 5): `begin_game`, `reduce`, `seat_to_act -> Option<PlayerId>` (SURFACE §6.1;
  `.expect`ed), `ReduceResult { state, events, error }` (destructured with `..`), `fold(&FoldArgs) ->
  FoldResult`, `FoldArgs: Default { seed: String, decks: (Vec<String>, Vec<String>), log: Vec<Action>, .. }`,
  `FoldResult { state, errors }` whose items have `nonce: String` and `error: String`, `hash_state`.
- `setup`: `mulligan_owed(&GameState) -> Vec<PlayerId>`, `mulligan_prompt_for(&GameState, PlayerId) ->
  Option<&PendingChoice>` (owned also fits).
- `catalog::registered_catalog()`, `scripts::registered_scripts()` (both `.clone()`d into an owned
  `CardDefs` / `IndexMap<String, CardScripts>`); the testkit's one-argument `register_catalog(CardDefs)` and
  `register_scripts(IndexMap<String, CardScripts>)` (SURFACE §8).
- `resolve` (part 3): `make_context(EngineSink<'a>, Option<CardInstance>, HookOptions) -> EffectContext<'a>`;
  `HookOptions: Default { controller: Option<PlayerId>, targets: Option<Vec<Selection>>, .. }`;
  `apply_effects(&[Effect], &mut EffectContext)`.
- `triggers::settle(&mut EngineSink, SettleOptions)` (`SettleOptions: Default`, TS's `options = {}`).
- `state_check`: `state_check(&mut EngineSink)`, `sacrifice_now(&mut EngineSink, &CardInstance)`.
- `damage`: `deal_damage(&mut EngineSink, DamageArgs { source: Option<CardInstance>, target: DamageTarget,
  amount: i32, flags: Option<_> }) -> i32`; `DamageTarget::{Unit { instance: CardInstance }, Hero { player:
  PlayerId }}`; `heal_hero`, `lose_health(&mut EngineSink, PlayerId, i32)`.
- `draw`: `draw_one(&mut EngineSink, PlayerId, Option<_ /* link */>)`, `draw(&mut EngineSink, PlayerId, i32)`.
- `replacements` (part 3): `answer_targeting(&mut EngineSink, AnswerTargetingArgs { target: CardInstance,
  by: PlayerId, what: TargetedWhat }) -> Option<CardInstance>`; `replacement_of(&IndexMap<String, Value>)
  -> Option<ReplacementRecord>` (TS took `{ data }`), with `ReplacementRecord { event: ReplacedEvent,
  flickered: Option<Vec<FlickeredCard>>, .. }` and `FlickeredCard: Serialize` (camelCase).
- `view_for::view_for(&GameState, PlayerId) -> PlayerView` (`Serialize`, compared as JSON).
- `combat` (part 4): `AttackTarget::{Unit { instance: CardInstance }, Hero { player: PlayerId }}` (built with
  exactly these fields, matched with `..`); `attack_targets(&GameState, &CardInstance) -> Vec<AttackTarget>`;
  `can_attack(&GameState, &CardInstance, &AttackTarget) -> bool`; `why_cannot_attack(..) -> Result<(),
  EngineError>`; `random_attack_targets(&GameState, &CardInstance, among) -> Vec<AttackTarget>` and
  `force_attacks_random(&mut EngineSink, &CardInstance, among, i32, Option<u32> /* since */)`, `among` built
  with `json_as(json!("enemyUnits"))` (any serde enum fits); `FORCED_RANDOM_WORK: &str`.
- `restrictions`: `effect_is_from_spell(&EffectContext) -> bool`, `unaffected_by(&EffectContext,
  &CardInstance) -> bool` (TS's `Pick<EffectContext, …>`).
- `work::owed_work(&GameState, Option<&str>) -> Vec<WorkItem>`.
- `layers`: `unit_view(&GameState, &CardInstance) -> UnitView { attack, max_health, health, keywords,
  position: Position, .. }`, `unclamped_attack(&GameState, &CardInstance) -> i32`.
- `zones` (part 2): `ZoneSlot` by value; `place_on_field(&mut GameState, CardInstance, ZoneSlot, options /*
  { stack? } */) -> bool`; `card_at(&GameState, ZoneSlot) -> Option<&CardInstance>`; `pile_at(&GameState,
  ZoneSlot) -> Option<&Pile>`; `lock_zone(&mut GameState, ZoneSlot)`; `move_to_zone(&mut GameState,
  &CardInstance, OffFieldZone, options: Default)`; `OffFieldZone::{Hand, Library, Graveyard, Exile}`;
  `active_units_of(&GameState, PlayerId)` (a `Vec`).
- `modifiers::schedule_delayed(&mut EngineSink, PlayerId, DelayedAt, Resume, Option<String>, Option<i32>)`,
  called with a hook's `ctx` (`&mut EffectContext` coerces to `&mut EngineSink`).
- `enchantments::has_enchantment(&CardInstance, &str /* Enchantment["kind"] */) -> bool` (no
  `EnchantmentKind` type exists in the frozen wire).
- `subsystems::rotation::{rotate_rings(&mut EngineSink, RotationArgs) -> RotationResult, RotationResult {
  moved, crossed, bounced: Vec<String> }}`, `RotationArgs: Deserialize` (`{ direction, perspective, radiant? }`).
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs: Default {
  ingredients: Vec<CardInstance>, target: Option<CardInstance>, .. }}`.
- `subsystems::ai_policy::{choose_action(&GameState, PlayerId, &mut Rng, PolicyOptions) ->
  Option<ActionBody>, PolicyOptions: Default}`.
- Effects (parts 6–7), every argument `Deserialize` from the TS literal (`json_as(json!(…))`, SURFACE §6.6):
  `effects::{shuffle_random_from_catalog, cast_rounds_until_death, damage_all, damage, destroy,
  forced_attack_own_hero, forced_attack_random, forced_attacks_on, plague, vanilla, discard, heal,
  choose_mode, draw}`.

## Decisions
- **Sinks.** `sinkFor(state)` is not called: a Rust fn cannot hand back a sink owning the rng and event
  list it borrows. Each file keeps TS's three parts side by side in a local `Bench { state, events, rng }`
  (`Bench::sink_for(state)` = an empty event list and `Rng::new(&state.seed, state.rng_cursor)`, as
  `sinkFor` did) and builds an `EngineSink` over them per engine call (`b.sink()`); `converting` and
  `dry_running` are at rest between calls, so this is TS's one sink. Where TS wrote the cursor back
  (`state.rngCursor = sink.rng.cursor`), so does the port; nowhere else.
- **Contexts.** TS built one `makeContext(sink, self)` and reused it with a live `self`; the port builds a
  fresh context per `applyEffects` with `self` read from the state as it stands then (`Bench::apply`).
- **Live objects.** Cards are held as owned copies for their ids, read back with `find_instance` (`live`)
  and written with `find_instance_mut` (`live_mut`) — every TS write through a live object and every read
  of one is made on the card in the state.
- **Recorders.** `recorder(state)` takes the state. Where TS kept using `state` after making a recorder,
  that `state` was still the pre-play state (`reduce` clones), so the port passes `state.clone()`
  (restrictions' immune and conditional tests). replacements' last test put the Mend into the recorder's
  live state right after making it; the port puts it in just before (no id or rng draw differs, and the
  recorder's start copy is never read there).
- **`createGame` refusals** panic with TS's message (part 1); `state.rs` catches the panic
  (`catch_unwind`) and matches the text. `toThrow(/a.*b/)` is "`a` occurs, then `b` after it".
- **Literals.** Card defs, effect arguments, actions (`json_as::<ActionInput>`/`Action`), static flags and
  option bags are TS's object literals through `json_as(json!(…))`; `{ ...a, ...b }` on defs is a shallow
  key merge. TS module counters (`nextIndex`) are written out as the index each def got; module `let nonce`
  counters are `static AtomicU32`s. Card-def constants are local zero-argument fns.
- **Expectations.** `toEqual` on events, views, records and results compares serde JSON; `toMatchObject` on
  one event checks its listed keys; `expect.any(Number)` is `is_number()` with the key count checked.
- **TS default and optional parameters**: a defaulted scalar is a trailing `Option` (`None` when omitted);
  an options object is `Default::default()` or `json_as(json!({…}))`. Local helpers keep TS's shape except
  setup-aside's `start`, which returns only `begun` (no test reads its `game`), and rounds' `casted`, which
  drops `before` (no test reads it).
- **replay-scripted's cached run** is a `static OnceLock<Run>`: computed once by whichever test thread gets
  there first, which registers the cards itself; the result is plain data.
- **Names**: an `R<n>` anywhere in a title is a leading `r<n>_` token, several in title order (an R-id cited
  only in a comment is not added); `§x.y` is `section_x_y_` at the front or `x_y` inside; `#52` is `c52`.
- **Exposure**: read part 1's frozen files (`state.rs`, `script.rs`, `config.rs`, `rng.rs`, `prelude.rs`,
  `lib.rs`, `testkit/mod.rs`, `wire/{actions,catalog_types,events,view}.rs`, the `mod.rs` files) and the
  notes of parts 1, 24.1, 25.1 and 26.1. No other Rust under `crates/` was opened.
