# Slice: part 24, chunk 4 of 7 — engine tests 1 (effect verbs), files 31–42 (#412, parent #306)
BUILDS-RUN: 0

## FILES
`crates/engine/tests/rules/{effects_move, effects_perks, effects_plague_random_cast, effects_plague,
effects_plus_c, effects_radiant, effects_random, effects_reveal, effects_shuffle_card, effects_split,
effects_statuses, effects_steal}.rs` — every TS `it` (138) and `describe` (29) ported in TS order, the
header comments and every ruling/§ comment kept. Notes: this file, `part-24-4.assumptions`,
`spec-gaps-part-24-4.md`. Nothing left.

## SURFACE
Matched §4 (paths, names, types), §6.5/§6.6 (`EngineSink`, `Effect.apply` called as
`(effect.apply)(&mut ctx)`), §7.3 (test names), §8 (testkit: `register_catalog`, `register_scripts`,
`json_as`, `json!`). Part 1's frozen types used as written (`CardInstance`, `GameState`, `GameEvent`,
`Selection`, `Zone`, `AnnounceRecord`, `Exertion`, `Counters`, `AttackHealth`, `Script`, `CardScripts`,
`EffectContext`, `Rng`).

## GAPS
Tests not ported: none (`spec-gaps-part-24-4.md` lists the assertions written against the nearest
Rust observable).

Names called that another part provides (module path; the shape these files assume):

Fixtures (part 24, chunks 6–7):
- `rules::fixtures::harness`: `new_game(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) ->
  GameState`; `put(&mut GameState, def_id: &str, at: ZoneSlot, options: Value) -> CardInstance` (TS's
  `{ radiant? }` bag as JSON, always passed, `json!({})` when TS omitted it; answers a copy of the placed
  card); `slot(PlayerId, Row, lane: i32) -> ZoneSlot`; `in_hand(&mut GameState, def_id: &str, PlayerId,
  count: i32) -> Vec<CardInstance>` (count always passed); `set_library(&mut GameState, PlayerId,
  &[&str]) -> Vec<CardInstance>`. Not called: `sinkFor` and `eventsOfType` (see Decisions).
- `rules::fixtures::catalog`: `unit_def(index: i32, overrides: Value) -> CardDef`, `spell_def(index: i32,
  overrides: Value) -> CardDef`, `token_def(name: &str, tags: Vec<Tag>) -> CardDef`,
  `vanilla_catalog(count: i32, from: i32) -> CardDefs`.
- Card fixtures, each TS `export const x: CardDef` as `pub fn x() -> CardDef`: `combat::{plain,
  stacker}`, `field::{doom, listener}`, `damage_combat::{bolt, bot_loser, grunt, rattle, snake, wall,
  warded}`, `generation::{big_body, body, charger, crawler, dusting, outbreak, plague_book, quiet_trap,
  scatter, slime, toxins}`.
- `field::playing(&str) -> GameState`; `damage_combat::playing(&str) -> GameState`,
  `damage_combat::recorder(&GameState) -> Recorder` with pub fields `start: GameState`, `log:
  Vec<Action>` and methods `play(&mut self, body) -> ReduceResult`, `state(&self) -> &GameState`;
  `damage_combat::replays_to(&GameState, &[Action], &GameState) -> bool`.
- `generation::{Run { pub start: String, pub log: Vec<Action>, pub state: GameState }, playing(&str) ->
  Run, frozen(&Run) -> Run, act(&Run, body) -> Run, refusal(&Run, body) -> Option<String>, answer(&Run,
  Selection, Option<PlayerId>) -> Run, replayed(&Run) -> GameState, hand_card(&mut GameState, &str,
  PlayerId) -> CardInstance}`. The run functions take `&Run` (SURFACE §6.5: TS's never mutated their
  argument). Every action body is built with `json_as(json!(TS literal))`, so a fixture taking the
  wire's `ActionInput` or a `Value` both compile.

Engine (parts 2–8), as their owners' notes give them where they do:
- `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext,
  HookOptions { controller: Option<PlayerId>, targets: Option<Vec<Selection>>, .. }: Default,
  apply_effects(&[Effect], &mut EffectContext)}` — part 3.2's shape (the owner's notes). Part 24.1 and
  6.2 called `make_context(EngineSink, …)` by value; part 31 picks one.
- `state_check::state_check(&mut EngineSink)`; `triggers::settle(&mut EngineSink, SettleOptions)`
  (`Default::default()`).
- `reduce::{reduce(&GameState, &Action), begin_game(&GameState), legal_actions(&GameState, PlayerId),
  ReduceResult}`; `replay::hash_state`; `view_for::{view_for, HIDDEN_ID, HIDDEN_OPTION_LABEL}`.
- `catalog::registered_catalog()` and `scripts::registered_scripts()` (either a reference or an owned
  copy: both are `.clone()`d); the testkit's `register_catalog(CardDefs)`, `register_scripts(IndexMap<
  String, CardScripts>)`.
- `zones::{card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>, place_on_field(&mut GameState, &mut
  CardInstance, &ZoneSlot, PlaceOnFieldOptions { stack: Option<bool> }: Default) -> bool, move_to_zone(
  &mut GameState, &mut CardInstance, OffFieldZone::Graveyard, Default::default()), lock_zone(&mut
  GameState, &ZoneSlot), active_units_of(&GameState, PlayerId)}` (part 2.1's shapes; a slot is passed
  as `&slot(..)`, which its `impl Into<ZoneSlot>` takes).
- `layers::{unit_view(&GameState, &CardInstance) -> UnitView { attack, max_health, health, .. },
  unit_has(&GameState, &CardInstance, KeywordKind)}`.
- `plague::{plague_on(&CardInstance) -> i32, plague_multiplier_of(&GameState, &CardInstance) -> i32,
  place_plague_on(&mut EngineSink, &CardInstance, i32) -> i32, remove_plague(same) -> i32,
  permanents_on_field(&GameState, Option<PlayerId>), plague_on_field(&GameState, Option<PlayerId>) ->
  i32}` (`None::<PlayerId>` for TS's omitted argument).
- `damage::{hero_hit_amount(&GameState, PlayerId, i32, pierce: bool) -> i32, DamageTarget::Hero {
  player }}`; `combat::can_attack(&GameState, &CardInstance, &DamageTarget) -> bool`;
  `restrictions::is_berserk(&CardInstance)`; `traps::is_spent(&GameState, &CardInstance)`;
  `mana::effective_cost(&GameState, &CardInstance, Default::default())`;
  `play_choices::gifted_makes_radiant(&GameState, PlayerId, i32) -> bool`;
  `announce::begin_announce(&mut GameState, AnnounceRecord)`;
  `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs { ingredients:
  Vec<CardInstance>, target: Option<CardInstance>, .. }: Default}`.
- Effects, data arguments built with `json_as(json!(TS literal))`: `exile`, `bounce`, `discard`,
  `discard_random`, `counter`, `damage`, `gain_hero_armor`, `discount_random_in_hand`, `place_plague`,
  `place_plague_each`, `place_plague_random`, `place_plague_tokens`, `consume_plague`,
  `effects::counters::plague` (named by module: `effects::plague` is also a module), `set_radiant`,
  `set_radiant_random`, `radiant_chance`, `shuffle_card_into`, `damage_split`, `go_berserk`,
  `may_attack_again`, `vanilla`, `steal`, `flip_coins`, `flip_coin_keyword`, `fuse_cards`, `rotate`,
  `damage_rounds_until_death`, `choose_mode`; TS's `= {}` defaults as `Default::default()`: `reveal`,
  `counter`, `discard_random`, `steal`, `steal_all`, `set_radiant`.
- Effect arguments holding a function, by struct literal (part 6.2's names): `ChooseFromHandArgs {
  where_: Option<Arc<dyn Fn(&EffectContext, &CardInstance) -> bool + Send + Sync>>, ..json_as(…) }` (the
  rest must deserialise); `CastRandomArgs { query: CastRandomQuery::Fixed(CatalogQueryArgs), count:
  Option<CastRandomCount::Fixed(i32)>, radiant: Option<bool>, how: <deserialisable> }`; `CastNewArgs {
  def: CastNewDef::from(String), radiant: Option<bool>, how: CastHow (deserialisable) }`. Part 6.2
  says these derive `Clone` only, with the `how` half flattened in; if their fields differ, part 31 fixes
  these three call sites.

## Decisions
- `sinkFor(state)` cannot hand back a sink that owns its rng, so each file builds one from the frozen
  types: `Rng::new(&state.seed, state.rng_cursor)` + `EngineSink::new`. The cursor is written back where
  TS wrote it back (`state.rngCursor = sink.rng.cursor`) and nowhere else. Where TS kept one sink across
  several runs (effects-plague), a local `sink_for!` macro opens it for the rest of the block.
- `put`, `in_hand`, `set_library`, `hand_card` answer copies; TS read and wrote the live objects they
  returned. Every read after a change goes back to the state by id (`find_instance`), every write
  before a run goes through `find_instance_mut`. Same assertions, same values.
- Events are compared as their serialised JSON (`of_type(events, "kind")`, a local filter on `"type"`),
  so TS's `toEqual` literals port key for key and absent optionals compare as TS's `undefined`; views
  likewise. `toMatchObject` is a local `matches_object` (objects by subset, arrays by length and element).
- Selections are built from the frozen `Selection::Instance` (generation's `pick` is a local `pick`).
- Fixture cards defined in a test file (`unitDefOf`, `def`, `spell`) are built with `json_as::<CardDef>`
  from the TS literal; TS's module index counters (`nextIndex`) are written out as each def's number.
- TS default parameters are passed explicitly (`new_game(seed, None)`, `in_hand(…, 1)`,
  `vanilla_catalog(40, 1)`, `hero_hit_amount(…, false)`).
- effects-plus-c's module nonce counter (`pcv<n>`) is the game's own `applied.len() + 1` (unique within a
  game, below `NONCE_HISTORY`; the hash leaves `applied` out, §5.2), not a static.
- `expect(() => act(...)).toThrow(/re/)` in effects-plague is `generation::refusal(&run, body)` (the same
  reduce's error) with `contains`.
- Names: every R-id of a title is a leading `r<n>_` token in title order; `§x.y` is `sx_y`; `#N` is `cN`;
  `C+` is `c_plus`; a `describe` is a `mod` named by the same rules.
