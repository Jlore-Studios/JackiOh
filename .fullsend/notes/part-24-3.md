# Slice: part 24 (engine tests 1: effect verbs and fixtures), chunk 3 of 7 (#412)
BUILDS-RUN: 0

## FILES
All new, each the whole TS file (every `describe` as a `mod`, every `it` as a `#[test]`, in TS order,
with the header comment and every comment that states a rule or cites a ruling):
`crates/engine/tests/rules/effects_{destroy, draw_while, each, enchant, flicker, fruit, give,
hand_exile, heal, health, library, locks}.rs` ← `packages/engine/test/effects-{destroy, drawWhile,
each, enchant, flicker, fruit, give, hand-exile, heal, health, library, locks}.test.ts`.
138 of 138 `it`s, 34 of 34 `describe`s; nothing dropped. `.fullsend/notes/spec-gaps-part-24-3.md` lists
the assertions written against a nearer observable than TS's live object.

## SURFACE
§4.1 paths, §4.2 names, §7.3 test names (every `R<n>` of a title leads, in title order:
`r443_r78_r215_…`), §8's `json!` literals and `testkit::{register_catalog, register_scripts}`.
Part 1 wins where it differs: hooks take `&mut EffectContext`, `EngineSink::new(state, events, rng)`.

## GAPS

### Not ported
Nothing.

### Names called, expected from other parts (TS name snake_cased at its TS module's path; the shape I assumed)
Fixtures (part 24, other chunks):
- `fixtures::harness::{new_game(&str, Option<_ decks>) -> GameState, put(&mut GameState, &str, ZoneSlot)
  -> CardInstance, slot(PlayerId, Row, i32) -> ZoneSlot, in_hand(&mut GameState, &str, PlayerId, i32) ->
  Vec<CardInstance>, set_library(&mut GameState, PlayerId, &[String]) -> Vec<CardInstance>,
  events_of_type(&[GameEvent], GameEventType) -> Vec<&GameEvent>}` (the element only needs `Serialize`
  and `.len()`). TS's `sinkFor` is not called: an `EngineSink` borrows the state, so each file keeps a
  private `Sink { events, rng }` that lends it the state call by call (Decisions).
- `fixtures::catalog::{token_def(&str, &[Tag]) -> CardDef (TS default tags written out), vanilla_catalog(i32, i32) -> CardDefs}`.
- `fixtures::combat::{plain, stacker, big_body, indestructible}() -> CardDef`, `combat_catalog(CardDefs) ->
  CardDefs`, `combat_scripts() -> IndexMap<String, CardScripts>` (TS `COMBAT_SCRIPTS`).
- `fixtures::scripts::{fixture_catalog(CardDefs) -> CardDefs, fixture_scripts()}` (TS `FIXTURE_SCRIPTS`).
- `fixtures::fruit::{fruit_catalog(CardDefs) -> CardDefs, fruit_scripts(), cast_on_draw(), grape_roller(),
  mythic(), priced_draw(), replacer()}`.
- `fixtures::field::{playing(&str) -> GameState, act(&GameState, <ActionInput>) -> GameState,
  act_result(&GameState, <ActionInput>) -> ReduceResult, flush(&mut GameState, PlayerId, i32 /* TS
  default 10 */), notes_of(Option<&CardInstance>) -> Vec<String>, blink, spatula, tesla, watcher,
  banner, doom, eater, leak, lockdown, unlocker}`. Action bodies are passed as `json_as(json!(…))`, so
  any `DeserializeOwned` parameter type (`ActionInput`, `ActionBody`, `Value`) compiles.
- `fixtures::instance_data::{instance_game(&str) -> GameState, body(), echo_bolt()}`.
- `fixtures::prompts::{cast_on_draw_marker, common_resources, fluffy_grip, glitch, grunt, income_tax}()
  -> CardDef, quickdraw_of(&CardDef) -> CardDef`.
- `fixtures::prompt_harness::{board(&str) -> GameState, cast_now(&mut GameState, &str, PlayerId, bool) ->
  Vec<GameEvent>` (TS returned the sink: here its events), `answer_keys(&mut GameState, &[&str])`
  returning a struct with `events: Vec<GameEvent>` and `error: Option<String>` (TS `{ sink, error }`),
  `open_as(&GameState, PromptKind, PlayerId) -> PendingChoice`, `round_trip(&GameState) -> GameState`,
  `act(&GameState, <ActionInput>, Option<&mut Vec<Action>>) -> GameState`, `replayable(&str, &[String],
  &[String])` returning a struct with fields `state`, `log: Vec<Action>`, `decks`, `expect_replays(&str,
  &decks, &[Action] /* &Vec */, &GameState)`, `hand_card(&GameState, PlayerId, &str)` (owned or `&`)}`.
- `fixtures::damage_combat::{playing(&str), recorder(&GameState)` returning a struct with fields `start:
  GameState`, `log: Vec<Action>` and methods `play(&mut self, <ActionInput>) -> ReduceResult`, `state()
  -> GameState or &GameState` (called as `&game.state()`), `replays_to(&GameState, &[Action], &GameState)
  -> bool, blood_moon, gambit, grunt, vital_kill}`.

Engine:
- `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext` (part
  3.2's shape; part 24.1 and 2.2/6.2 assumed `(EngineSink, Option<CardInstance>, …)`: part 31 picks one),
  `HookOptions { controller, targets, .. }: Default`, `apply_effects(&[Effect], &mut EffectContext)`,
  `lazy_part(&'static str, |ctx, memo| EffectPart)}`.
- `state_check::state_check(&mut EngineSink)`; `triggers::settle(&mut EngineSink, SettleOptions: Default)`.
- `prompts::{run_hook_resumable(&mut EngineSink, &CardInstance, &str, <options>: Default),
  answer_prompt(&mut EngineSink, &AnswerInput) -> Result<(), EngineError>, AnswerInput { player_id,
  choice_id, selection }}`.
- `zones::{ZoneSlot, card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>, place_on_field(&mut
  GameState, &CardInstance, &ZoneSlot, <options>: Default + Deserialize from `{ stack }`) -> bool,
  move_to_zone(&mut GameState, &CardInstance, OffFieldZone, <options>: Default), OffFieldZone::{Hand,
  Library, Graveyard, Exile}, is_buried(&GameState, &CardInstance), home_of(&GameState, &str) -> Option<_>,
  slots_of(PlayerId, Row) -> Vec<ZoneSlot>, is_locked(&GameState, &ZoneSlot)}`. Every card-taking call
  gets `&CardInstance` (the copy read from the state just before; parts 2.2, 3.1, 6.2's convention);
  part 3.2 assumed `&mut` for `place_on_field`/`move_to_zone`.
- `query::zone_cards(&GameState, PlayerId, ZoneName)`; `catalog::{registered_catalog(), find_def(None,
  &str) -> Option<&CardDef>, query(&CatalogQueryArgs) -> Vec<&CardDef>, pick_generated(&mut Rng,
  &[&CardDef], None) -> Option<&CardDef>}` (part 2.2's shapes); `scripts::registered_scripts()` (cloned).
- `enchantments::{enchantments_of(&CardInstance) (slice or Vec), enchantments_of_kind(&CardInstance,
  EnchantmentKind), has_enchantment(&CardInstance, EnchantmentKind), add_enchantment(&mut CardInstance,
  &Enchantment) -> bool, united_enchantments(&[CardInstance]) -> Option<Vec<Enchantment>>, EnchantmentKind}`.
- `ownership::{take_into_hand(&mut EngineSink, &CardInstance, PlayerId) -> Option<_>, change_owner(…) ->
  bool, draw_from_library_of(&mut EngineSink, PlayerId, PlayerId, LibraryEnd: Default) -> Option<_>}`.
- `animated::animate_card(&mut EngineSink, &CardInstance, <options>: Default)`; `modifiers::schedule_delayed(
  &mut EngineSink, PlayerId, DelayedAt, Resume, Option<String>, Option<i32>)`; `stays::{exit_mark(&GameState)
  -> u32, left_field_after(&GameState, u32, &str) -> bool}`; `view_for::{view_for, HIDDEN_ID}`;
  `replay::hash_state`; `mana::effective_cost(&GameState, &CardInstance, CostOptions: Default) -> i32`;
  `layers::unit_view(&GameState, &CardInstance).health`; `damage::{deal_damage(&mut EngineSink,
  DamageArgs { source, target, amount, flags }), DamageTarget::Hero { player }, heal_hero(&mut EngineSink,
  PlayerId, i32)}`; `draw::draw(&mut EngineSink, PlayerId, i32)`; `play_choices::legal_zones_for(&GameState,
  PlayerId, &CardInstance, &[String]) -> Vec<ZoneChoice>`; `subsystems::fuse::{fuse(&mut EngineSink,
  FuseArgs), FuseArgs { ingredients: Vec<CardInstance>, target: Option<CardInstance>, .. }: Default}`.
- Effects, every argument built from TS's literal with `json_as(json!(…))` (TS `= {}` passed as
  `Default::default()`): `damage`, `destroy::{destroy, sacrifice}`, `draw_while(DrawWhileArgs { more:
  Arc<dyn Fn(&EffectContext) -> bool + Send + Sync> })`, `for_each_card(ForEachCardArgs { cards:
  Arc<dyn Fn(&EffectContext) -> Vec<String>>, each: Arc<dyn Fn(&str) -> Effect> })` (as part 24.1),
  `choose_mode`, `draw_from_library`, `enchant`, `shuffle_copies_of_self`, `summon_copy`, `transform`,
  `flicker::{flicker, flicker_card(&mut EngineSink, &CardInstance) -> bool}`, `steal::steal`,
  `fruit::{add_rolled_grapes, damage_enemy_or_heal_friend, draw_priced, replace_hand_with_random,
  roll_grape(&mut Rng, i32)}` (`roll_grape` through fruit's re-export of catalog's, as TS imports it),
  `draw_from_opponent`, `give_from_hand`, `take_from_library`, `hand_exile::exile_random_from_hand`,
  `heal::heal`, `convert_healing`, `set_health`, `add_to_hand::{add_random_from_catalog, add_to_hand}`,
  `choose::{discover_from_catalog, discover_from_library}`, `library::{exile_bottom_of_library,
  exile_random_from_library}`, `counters::{lock, unlock}`, `locks::{lock_lane, lock_own_zone() (no
  argument, as TS), lock_played_zone, lock_random_zone, unlock_all}`.

## Decisions
- **Live objects.** TS read and wrote cards through the objects its helpers handed back. Every read
  here is by id from the state (`find_instance`), every write through `find_instance_mut`, and every
  engine call that took the live object gets the card's copy read from the state just before the call.
  A card in no pile cannot be read back (spec gaps).
- **The sink.** TS's `sinkFor(state)` held the state; an `EngineSink` borrows it. Each file keeps a
  private `Sink { events, rng }` (the rng from `(state.seed, state.rng_cursor)`) that lends itself and
  the state to an `EngineSink` call by call, so a test reads the state between calls as TS did and the
  events and rng stay the one sink's. The cursor is written back exactly where TS wrote it (destroy's
  runner, fruit's, hand-exile's, give's `run`), never elsewhere. One TS context reused for several
  effects stays one Rust context (locks, health); the state is read through `ctx.state` meanwhile.
- **Matchers.** `toEqual` on events: JSON equality (`skip_serializing_if` makes absent absent, as
  `toEqual` ignores `undefined`). `toMatchObject`: a private `matches_object` over JSON (objects by
  subset, arrays by length and element). `expect.any(String)`: the field checked as a string and
  removed before comparing. `.map((e) => e.<key>)`: a private `pluck` over the events' JSON.
- **Names.** §7.3: every `R<n>` of a title leads, in title order; `§` numbers and BUILD ids are
  dropped from the name, and the TS title is kept in a `///` line wherever the name loses part of it.
  `#19`/`#40`-style card numbers stay as trailing words.
- **Fixture defs local to a TS file** (destroy's `unitDefOf`, library's `makeDef`, drawWhile's `spell`,
  each's `drawer`) are private fns with TS's `nextIndex` written out per def in declaration order, and
  TS's `{ ...literal, ...overrides }` as a shallow JSON merge.
- `effects-each`'s `asksAfter` closure: `Arc<OnceLock<String>>` (no `Mutex`/`RefCell`, clippy.toml).
- `effects-library`'s `run` passes TS's `options` as `HookOptions { controller }` (TS handed `makeContext`
  the whole options object, whose `self` key it ignores).
